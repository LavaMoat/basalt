use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit, VisitWith};

use crate::analysis::{ExportAnalysis, ImportAnalysis, ExportRecord};

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

        //println!("Imports {:#?}", imports);
        //println!("Exports {:#?}", exports);

        for (key, symbols) in imports.iter() {
            let words = symbols.iter()
                .map(|s| s.word())
                .collect::<Vec<_>>();
            record.imports.insert(key.clone(), words);
        }

        for symbol in exports.iter() {
            match symbol {
                ExportRecord::All { module_path } => {
                    record.imports.insert(module_path.clone(), vec![]);
                    record.export_alls.push(module_path.clone());
                }
                ExportRecord::Decl { decl: _ } => {
                    // TODO: handle export declarations
                }
                ExportRecord::DefaultExpr { expr: _ } => {
                    // TODO: handle export expression declarations
                }
                ExportRecord::NamedSpecifier { orig: _, exported: _ } => {
                    // TODO: handle named specifiers
                }
            }
        }

        /*
        if let Some(module) = bundler
            .load_transformed(&file_name, true)
            .context("load_transformed failed")?
        {

            for spec in module.exports.reexports.iter() {
                let module_path = format!("{}", spec.0.src.value);
                if spec.1.is_empty() {
                    record.imports.insert(module_path.clone(), vec![]);
                    record.export_alls.push(module_path);
                } else {
                    let words = collect_words(&spec.1);
                    record.imports.insert(module_path.clone(), words);

                    // Question: is this the correct way to represent multiple specifiers in the
                    // live export map, add a new entry for each specifier?
                    for s in spec.1.iter() {
                        match s {
                            Specifier::Specific { local, alias } => {
                                let key = format!("{}", local.sym());
                                let alias = if let Some(alias) = alias {
                                    format!("{}", alias.sym())
                                } else {
                                    key.clone()
                                };
                                let value = (alias, false);
                                record.live_export_map.insert(key, value);
                            }
                            Specifier::Namespace { .. } => {
                                todo!()
                            }
                        }
                    }
                }
            }
        }
        */

        let (fixed, live) = self.analyze_exports(&module);
        record.fixed_export_map.extend(fixed);
        record.live_export_map.extend(live);

        Ok(record)
    }

    fn analyze_exports(
        &self,
        module: &Module,
    ) -> (HashMap<String, Vec<String>>, HashMap<String, LiveExport>) {
        let mut v = ExportDetector {
            fixed: HashMap::new(),
            live: HashMap::new(),
        };
        module.visit_children_with(&mut v);
        (v.fixed, v.live)
    }
}

struct ExportDetector {
    fixed: HashMap<String, Vec<String>>,
    live: HashMap<String, LiveExport>,
}

impl Visit for ExportDetector {
    fn visit_export_default_expr(
        &mut self,
        _n: &ExportDefaultExpr,
        _: &dyn Node,
    ) {
        self.fixed
            .insert(String::from("default"), vec![String::from("default")]);
    }

    fn visit_export_decl(&mut self, n: &ExportDecl, _: &dyn Node) {
        //println!("Export decl {:#?}", n);
    }
}
