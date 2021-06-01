use std::path::Path;
use std::sync::Arc;
use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};

use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};
use swc::{config::Options, Compiler};
use swc_bundler::{
    Load,
    Resolve,
    bundler::load::{Specifier},
};
use swc_common::{FileName/*, SourceMap */};

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
        let (_source_map, compiler) = crate::bundler::get_compiler();
        let options: Options = Default::default();
        Parser {
            loader: Box::new(SwcLoader::new(Arc::clone(&compiler), options)),
            resolver: Box::new(NodeResolver::new()),
            compiler,
        }
    }

    pub fn load<P: AsRef<Path>>(
        &self,
        file: P,
    ) -> Result<StaticModuleRecord> {
        let mut record: StaticModuleRecord = Default::default();

        let file_name = FileName::Real(file.as_ref().to_path_buf());
        let bundler = crate::bundler::get_bundler(
            Arc::clone(&self.compiler),
            self.compiler.globals(),
            &self.loader,
            &self.resolver,
        );

        //log::info!("Transform {}", file.as_ref().display());

        if let Some(module) = bundler
            .load_transformed(&file_name, true)
            .context("load_transformed failed")? {
            //println!("Module {:#?}", module);

            for spec in module.imports.specifiers.iter() {
                let module_path = format!("{}", spec.0.src.value);
                let words =
                    spec.1.iter()
                    .map(|s| {
                        match s {
                            Specifier::Specific {local, alias} => {
                                if let Some(alias) = alias {
                                    format!("{}", alias.sym())
                                } else {
                                    format!("{}", local.sym())
                                }
                            }
                            Specifier::Namespace {local, all} => {
                                if *all {
                                    String::from("*")
                                } else {
                                    format!("{}", local.sym())
                                }
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                record.imports.insert(module_path, words);
            }
        }

        Ok(record)
    }
}

