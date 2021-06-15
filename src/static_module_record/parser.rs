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

use super::StaticModuleRecord;

/// Parses a module to a static module record.
pub struct Parser {
    importer: ImportAnalysis,
    exporter: ExportAnalysis,
}

impl Parser {
    /// Create a new parser.
    pub fn new() -> Self {
        let importer = ImportAnalysis::new();
        let exporter = ExportAnalysis::new();
        Parser { importer, exporter }
    }

    /// Parse a module to a static module record.
    pub fn parse<'a>(&'a mut self, module: &'a Module) -> Result<StaticModuleRecord<'a>> {
        let mut record = StaticModuleRecord{
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

        let export_names = self.exporter.var_export_names();
        let mut live_exports = LiveExportAnalysis::new(export_names);
        module.visit_children_with(&mut live_exports);

        for (key, symbols) in self.importer.imports.iter() {
            //let mut names = symbols
                //.iter()
                //.map(|s| match s {
                    //ImportRecord::Named { local, .. } => local.clone(),
                    //ImportRecord::Default { local, .. } => local.clone(),
                    //ImportRecord::All { local } => local.clone(),
                //})
                //.collect::<Vec<_>>();

            let mut names = symbols
                .iter()
                .map(|s| match s {
                    ImportRecord::Named { local, .. } => local.as_ref(),
                    ImportRecord::Default { local, .. } => local.as_ref(),
                    ImportRecord::All { local } => local.as_ref(),
                })
                .collect::<Vec<_>>();


            let words = symbols.iter().map(|s| s.word()).collect::<Vec<_>>();
            record.imports.insert(key.clone(), words);

            record.import_alias.insert(key.clone(), names.clone());

            record.import_decls.append(&mut names);
        }

        for name in live_exports.live.iter() {
            record
                .live_export_map
                .insert(name.clone(), (name.clone(), true));
        }

        for symbol in self.exporter.exports.iter() {
            match symbol {
                ExportRecord::FnDecl { func } => {
                    let key = func.ident.sym.as_ref().to_string();
                    let val = key.clone();
                    record.fixed_export_map.insert(key, vec![val]);
                }
                ExportRecord::VarDecl { var } => match var.kind {
                    VarDeclKind::Const => {
                        for decl in var.decls.iter() {
                            match &decl.name {
                                Pat::Ident(ident) => {
                                    let key = ident.id.sym.as_ref().to_string();
                                    let val = key.clone();
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
                    record.fixed_export_map.insert(
                        String::from("default"),
                        vec![String::from("default")],
                    );
                }
                ExportRecord::Named { specifiers } => {
                    for spec in specifiers {
                        match spec {
                            ExportSpecifier::Named(export) => {
                                let key = format!(
                                    "{}",
                                    export
                                        .exported
                                        .as_ref()
                                        .unwrap_or(&export.orig)
                                        .sym
                                );
                                let val = format!("{}", export.orig.sym);
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
                                //format!("{}", export.orig.sym)
                            }
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>();

                    record
                        .import_alias
                        .insert(module_path.clone(), words.clone());

                    // TODO: make imports words a string slice!!!!
                    let words = words.iter().map(|s| s.to_string()).collect::<Vec<_>>();
                    record.imports.insert(module_path.clone(), words);

                    for spec in specifiers {
                        match spec {
                            ExportSpecifier::Named(export) => {
                                let key = format!(
                                    "{}",
                                    export
                                        .exported
                                        .as_ref()
                                        .unwrap_or(&export.orig)
                                        .sym
                                );
                                let val = format!("{}", export.orig.sym);
                                record
                                    .live_export_map
                                    .insert(key, (val, false));
                            }
                            _ => {}
                        }
                    }
                }
                ReexportRecord::All { module_path } => {
                    record.import_alias.insert(module_path.clone(), Vec::new());
                    record.imports.insert(module_path.clone(), Vec::new());
                    record.export_alls.push(module_path.clone());
                }
            }
        }

        Ok(record)
    }
}
