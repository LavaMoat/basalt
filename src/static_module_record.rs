//! Static module record.
//!
//! More information in the [static module record design document](https://github.com/endojs/endo/blob/master/packages/static-module-record/DESIGN.md).
use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use swc_ecma_ast::*;
use swc_ecma_visit::VisitWith;

use crate::analysis::{
    imports::ImportAnalysis,
    exports::{ExportAnalysis, ExportRecord, ReexportRecord},
    live_exports::LiveExportAnalysis,
};

/// Type for live exports.
pub type LiveExport = (String, bool);

/// Static module record that can be serialized to JSON.
#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StaticModuleRecord {
    /// All exports, eg: `export * from './foo.js';`
    pub export_alls: Vec<String>,
    /// All the imports for the module.
    pub imports: HashMap<String, Vec<String>>,
    /// Map of live exports.
    pub live_export_map: HashMap<String, LiveExport>,
    /// Map of fixed exports.
    pub fixed_export_map: HashMap<String, Vec<String>>,
}

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
            let words = symbols.iter().map(|s| s.word()).collect::<Vec<_>>();
            record.imports.insert(key.clone(), words);
        }

        for name in live_exports.live.iter() {
           record.live_export_map.insert(name.clone(), (name.clone(), true));
        }

        for symbol in exporter.exports.iter() {
            match symbol {
                ExportRecord::FnDecl { func } => {
                    let key = func.ident.sym.as_ref().to_string();
                    let val = key.clone();
                    record
                        .fixed_export_map
                        .insert(key, vec![val]);
                }
                ExportRecord::VarDecl { var } => {
                    match var.kind {
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
                    }
                }
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
