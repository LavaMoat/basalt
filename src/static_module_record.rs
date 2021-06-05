use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use swc_ecma_ast::*;
use swc_ecma_visit::VisitWith;

use crate::analysis::{
    ExportAnalysis, ExportRecord, ImportAnalysis, ReexportRecord,
};

pub type LiveExport = (String, bool);

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StaticModuleRecord {
    pub export_alls: Vec<String>,
    pub imports: HashMap<String, Vec<String>>,
    pub live_export_map: HashMap<String, LiveExport>,
    pub fixed_export_map: HashMap<String, Vec<String>>,
}

pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn load<P: AsRef<Path>>(&self, file: P) -> Result<StaticModuleRecord> {
        let mut record: StaticModuleRecord = Default::default();
        let (_, _, module) = crate::swc_utils::load_file(file)?;

        let mut importer = ImportAnalysis::new();
        module.visit_children_with(&mut importer);

        let mut exporter = ExportAnalysis::new();
        module.visit_children_with(&mut exporter);

        let imports = importer.imports;
        let exports = exporter.exports;
        let reexports = exporter.reexports;

        for (key, symbols) in imports.iter() {
            let words = symbols.iter().map(|s| s.word()).collect::<Vec<_>>();
            record.imports.insert(key.clone(), words);
        }

        for symbol in exports.iter() {
            match symbol {
                ExportRecord::VarDecl { var } => {
                    match var.kind {
                        VarDeclKind::Const => {
                            for decl in var.decls.iter() {
                                match &decl.name {
                                    Pat::Ident(ident) => {
                                        let key = format!("{}", ident.id.sym);
                                        let val = key.clone();
                                        record
                                            .fixed_export_map
                                            .insert(key, vec![val]);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        // let or var could be re-assigned so should we need to detect for
                        // assignments to determine if it goes in live_export_map
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
                        println!("Spec {:#?}", spec);
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

        for symbol in reexports.iter() {
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
