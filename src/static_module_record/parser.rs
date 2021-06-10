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
pub struct Parser {}

impl Parser {
    /// Create a new parser.
    pub fn new() -> Self {
        Parser {}
    }

    /// Load a module from a file and parse it.
    pub fn load<P: AsRef<Path>>(&self, file: P) -> Result<StaticModuleRecord> {
        let (_, _, module) = crate::swc_utils::load_file(file)?;
        self.parse(&module)
    }

    /// Parse a module to a static module record.
    pub fn parse(&self, module: &Module) -> Result<StaticModuleRecord> {
        let mut record: StaticModuleRecord = Default::default();

        let mut importer = ImportAnalysis::new();
        module.visit_children_with(&mut importer);

        let mut exporter = ExportAnalysis::new();
        module.visit_children_with(&mut exporter);

        let export_names = exporter.var_export_names();
        let mut live_exports = LiveExportAnalysis::new(export_names);
        module.visit_children_with(&mut live_exports);

        for (key, symbols) in importer.imports.iter() {
            let mut names = symbols
                .iter()
                .map(|s| match s {
                    ImportRecord::Named { local, .. } => local.clone(),
                    ImportRecord::Default { local, .. } => local.clone(),
                    ImportRecord::All { local } => local.clone(),
                })
                .collect::<Vec<_>>();
            record.import_decls.append(&mut names);

            let words = symbols.iter().map(|s| s.word()).collect::<Vec<_>>();
            record.imports.insert(key.clone(), words);
        }

        for name in live_exports.live.iter() {
            record
                .live_export_map
                .insert(name.clone(), (name.clone(), true));
        }

        for symbol in exporter.exports.iter() {
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

        for symbol in exporter.reexports.iter() {
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
                                format!("{}", export.orig.sym)
                            }
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>();
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
                    record.imports.insert(module_path.clone(), Vec::new());
                    record.export_alls.push(module_path.clone());
                }
            }
        }

        Ok(record)
    }
}
