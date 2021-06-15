//! Parse static module record meta data.
use std::path::Path;

use anyhow::Result;

use swc_ecma_ast::*;
use swc_ecma_visit::VisitWith;

use crate::analysis::{
    exports::{ExportAnalysis, ExportRecord, ReexportRecord},
    imports::{ImportAnalysis, ImportRecord},
    live_exports::LiveExportAnalysis,
};

use super::{ImportName, StaticModuleRecord};

/// Parses a module to a static module record.
pub struct Parser {
    importer: ImportAnalysis,
    exporter: ExportAnalysis,
    live_exports: LiveExportAnalysis,
}

impl Parser {
    /// Create a new parser.
    pub fn new() -> Self {
        let importer = ImportAnalysis::new();
        let exporter = ExportAnalysis::new();
        let live_exports = LiveExportAnalysis::new();
        Parser {
            importer,
            exporter,
            live_exports,
        }
    }

    /// Parse a module to a static module record.
    pub fn parse<'m>(
        &'m mut self,
        module: &'m Module,
    ) -> Result<StaticModuleRecord<'m>> {
        let mut record = StaticModuleRecord {
            module,
            export_alls: Default::default(),
            imports: Default::default(),
            live_export_map: Default::default(),
            fixed_export_map: Default::default(),
            import_decls: Default::default(),
            import_alias: Default::default(),
        };

        module.visit_children_with(&mut self.importer);
        module.visit_children_with(&mut self.exporter);

        self.live_exports.exports = self.exporter.var_export_names();
        module.visit_children_with(&mut self.live_exports);

        for (key, symbols) in self.importer.imports.iter() {
            let mut names = symbols
                .iter()
                .map(|s| match s {
                    ImportRecord::Named { local, .. } => local.as_ref(),
                    ImportRecord::Default { local, .. } => local.as_ref(),
                    ImportRecord::All { local } => local.as_ref(),
                })
                .collect::<Vec<_>>();

            let imports = symbols
                .iter()
                .map(|s| match s {
                    ImportRecord::Named { local, alias } => {
                        ImportName{
                        name: local.as_ref(),
                        alias: alias.as_ref().map(|s| &s[..]) }
                    },
                    ImportRecord::Default { local, .. } => ImportName{ name: local.as_ref(), alias: None},
                    ImportRecord::All { local } => ImportName{ name: local.as_ref(), alias: None},
                })
                .collect::<Vec<_>>();

            record.imports.insert(&key[..], imports);

            record.import_alias.insert(&key[..], names.clone());

            record.import_decls.append(&mut names);
        }

        for name in self.live_exports.live.iter() {
            record.live_export_map.insert(name, (name, true));
        }

        for symbol in self.exporter.exports.iter() {
            match symbol {
                ExportRecord::FnDecl { func } => {
                    let key = func.ident.sym.as_ref();
                    let val = key;
                    record.fixed_export_map.insert(key, vec![val]);
                }
                ExportRecord::VarDecl { var } => match var.kind {
                    VarDeclKind::Const => {
                        for decl in var.decls.iter() {
                            match &decl.name {
                                Pat::Ident(ident) => {
                                    let key = ident.id.sym.as_ref();
                                    let val = key;
                                    record
                                        .fixed_export_map
                                        .insert(key, vec![val]);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                },
                ExportRecord::DefaultExpr { expr: _ } => {
                    record.fixed_export_map.insert("default", vec!["default"]);
                }
                ExportRecord::Named { specifiers } => {
                    for spec in specifiers {
                        match spec {
                            ExportSpecifier::Named(export) => {
                                let key = export
                                    .exported
                                    .as_ref()
                                    .unwrap_or(&export.orig)
                                    .sym
                                    .as_ref();

                                let val = export.orig.sym.as_ref();
                                record.fixed_export_map.insert(key, vec![val]);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        for symbol in self.exporter.reexports.iter() {
            match symbol {
                ReexportRecord::Named {
                    module_path,
                    specifiers,
                } => {

                    // FIXME: remove this!!!
                    let words = specifiers
                        .iter()
                        .filter(|s| {
                            if let ExportSpecifier::Named(_) = s {
                                true
                            } else {
                                false
                            }
                        })
                        .map(|s| match s {
                            ExportSpecifier::Named(export) => {
                                export.orig.sym.as_ref()
                            }
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>();
                    record.import_alias.insert(&module_path[..], words);

                    let words = specifiers
                        .iter()
                        .filter(|s| {
                            if let ExportSpecifier::Named(_) = s {
                                true
                            } else {
                                false
                            }
                        })
                        .map(|s| match s {
                            ExportSpecifier::Named(export) => {
                                ImportName { name: export.orig.sym.as_ref(), alias:None }
                            }
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>();

                    record.imports.insert(
                        &module_path[..],
                        words
                    );

                    for spec in specifiers {
                        match spec {
                            ExportSpecifier::Named(export) => {
                                let key = export
                                    .exported
                                    .as_ref()
                                    .unwrap_or(&export.orig)
                                    .sym
                                    .as_ref();

                                let val = export.orig.sym.as_ref();
                                record
                                    .live_export_map
                                    .insert(key, (val, false));
                            }
                            _ => {}
                        }
                    }
                }
                ReexportRecord::All { module_path } => {
                    let module_path = &module_path[..];
                    record.imports.insert(module_path, Vec::new());
                    record.import_alias.insert(module_path, Vec::new());
                    record.export_alls.push(module_path);
                }
            }
        }

        Ok(record)
    }
}
