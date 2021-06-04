use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};
use swc::{config::Options, Compiler};
use swc_bundler::{Load, Resolve, TransformedModule};
use swc_bundler_analysis::specifier::Specifier;
use swc_common::{FileName, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit, VisitWith};

fn collect_words(specs: &Vec<Specifier>) -> Vec<String> {
    specs
        .iter()
        .map(|s| match s {
            Specifier::Specific { local, alias } => {
                if let Some(alias) = alias {
                    format!("{}", alias.sym())
                } else {
                    format!("{}", local.sym())
                }
            }
            Specifier::Namespace { local, all } => {
                if *all {
                    String::from("*")
                } else {
                    format!("{}", local.sym())
                }
            }
        })
        .collect::<Vec<_>>()
}

pub type LiveExport = (String, bool);

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StaticModuleRecord {
    pub export_alls: Vec<String>,
    pub imports: HashMap<String, Vec<String>>,
    pub live_export_map: HashMap<String, LiveExport>,
    pub fixed_export_map: HashMap<String, Vec<String>>,
}

pub struct Parser {
    compiler: Arc<Compiler>,
    resolver: Box<dyn Resolve>,
    loader: Box<dyn Load>,
}

impl Parser {
    pub fn new() -> Self {
        let (_source_map, compiler) = crate::swc_utils::get_compiler();
        let options: Options = Default::default();
        Parser {
            loader: Box::new(SwcLoader::new(Arc::clone(&compiler), options)),
            resolver: Box::new(NodeResolver::new()),
            compiler,
        }
    }

    pub fn load<P: AsRef<Path>>(&self, file: P) -> Result<StaticModuleRecord> {
        let mut record: StaticModuleRecord = Default::default();

        //let file_name = FileName::Real(file.as_ref().to_path_buf());

        let module = crate::swc_utils::load_file(file)?;

        /*
        if let Some(module) = bundler
            .load_transformed(&file_name, true)
            .context("load_transformed failed")?
        {
            for spec in module.imports.specifiers.iter() {
                let module_path = format!("{}", spec.0.src.value);
                let words = collect_words(&spec.1);
                record.imports.insert(module_path, words);
            }

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

            let (fixed, live) = self.analyze_exports(&module);
            record.fixed_export_map.extend(fixed);
            record.live_export_map.extend(live);
        }
        */

        Ok(record)
    }

    fn analyze_exports(
        &self,
        transformed: &TransformedModule,
    ) -> (HashMap<String, Vec<String>>, HashMap<String, LiveExport>) {
        let mut v = ExportDetector {
            fixed: HashMap::new(),
            live: HashMap::new(),
        };
        transformed
            .module
            .visit_with(&Invalid { span: DUMMY_SP } as _, &mut v);
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
        println!("Export decl {:#?}", n);
    }
}
