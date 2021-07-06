//! Parse static module record meta data.
use anyhow::Result;

use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{VisitAllWith, VisitWith};

use indexmap::IndexSet;

use super::analyzer::{
    Analyzer, ExportRecord, ImportRecord, LiveExportAnalysis, ReexportRecord,
};

use super::{ImportKind, ImportName, StaticModuleRecord};

use crate::helpers::var_symbol_names;

/// Parses a module to a static module record.
pub struct Parser {
    analyzer: Analyzer,
    live_exports: LiveExportAnalysis,
}

impl Parser {
    /// Create a new parser.
    pub fn new() -> Self {
        let analyzer = Analyzer::new();
        let live_exports = LiveExportAnalysis::new();
        Parser {
            analyzer,
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
            hoisted_funcs: Default::default(),
            hoisted_refs: Default::default(),
        };

        module.visit_children_with(&mut self.analyzer);

        self.live_exports.exports = self.analyzer.var_export_names();
        module.visit_all_children_with(&mut self.live_exports);

        record.hoisted_funcs = self
            .analyzer
            .hoisted_funcs
            .drain(..)
            .collect::<IndexSet<_>>();

        record.hoisted_refs = self
            .live_exports
            .hoisted_refs
            .drain(..)
            .collect::<IndexSet<_>>();

        for (key, symbols) in self.analyzer.imports.iter() {
            let imports = symbols
                .iter()
                .map(|s| match s {
                    ImportRecord::None => None,
                    ImportRecord::Named { name, alias } => Some(ImportName {
                        name: name.as_ref(),
                        alias: Some(&alias[..]),
                        kind: ImportKind::Named,
                    }),
                    ImportRecord::Default { name, .. } => Some(ImportName {
                        name: name.as_ref(),
                        alias: None,
                        kind: ImportKind::Default,
                    }),
                    ImportRecord::All { name } => Some(ImportName {
                        name: name.as_ref(),
                        alias: None,
                        kind: ImportKind::All,
                    }),
                })
                .filter(|s| s.is_some())
                .map(|s| s.unwrap())
                .collect::<Vec<_>>();

            record.imports.insert(&key[..], imports);
        }

        for symbol in self.analyzer.exports.iter() {
            match symbol {
                ExportRecord::FnDecl { func } => {
                    let key = func.ident.sym.as_ref();
                    let val = key;
                    record.fixed_export_map.insert(key, vec![val]);
                }
                ExportRecord::VarDecl { var } => {
                    let names = var_symbol_names(var)
                        .iter()
                        .map(|v| v.1.clone())
                        .flatten()
                        .collect::<Vec<_>>();
                    for name in names {
                        record.fixed_export_map.insert(name, vec![name]);
                    }
                }
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

        for name in self.live_exports.live.iter() {
            record.live_export_map.insert(name, (name, true));
            record.fixed_export_map.remove(&name[..]);
        }

        for symbol in self.analyzer.reexports.iter() {
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
                            ExportSpecifier::Named(export) => ImportName {
                                name: export.orig.sym.as_ref(),
                                alias: export
                                    .exported
                                    .as_ref()
                                    .map(|ident| ident.sym.as_ref()),
                                kind: ImportKind::Named,
                            },
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>();

                    record.imports.insert(&module_path[..], words);

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
                    record.export_alls.push(module_path);
                }
            }
        }

        Ok(record)
    }
}
