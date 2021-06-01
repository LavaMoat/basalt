use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};
use swc::{config::Options, Compiler};
use swc_bundler::{
    TransformedModule,
    bundler::load::Specifier, Load, Resolve};
use swc_common::FileName;

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
        let (_source_map, compiler) = crate::bundler::get_compiler();
        let options: Options = Default::default();
        Parser {
            loader: Box::new(SwcLoader::new(Arc::clone(&compiler), options)),
            resolver: Box::new(NodeResolver::new()),
            compiler,
        }
    }

    pub fn load<P: AsRef<Path>>(&self, file: P) -> Result<StaticModuleRecord> {
        let mut record: StaticModuleRecord = Default::default();

        let file_name = FileName::Real(file.as_ref().to_path_buf());
        let bundler = crate::bundler::get_bundler(
            Arc::clone(&self.compiler),
            self.compiler.globals(),
            &self.loader,
            &self.resolver,
        );

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
                }
            }

            let (fixed, live) = self.analyze_exports(&module);
            record.fixed_export_map = fixed;
            record.live_export_map = live;
        }

        Ok(record)
    }

    fn analyze_exports(
        &self,
        module: &TransformedModule,
    ) -> (HashMap<String, Vec<String>>, HashMap<String, LiveExport>) {
        let fixed = HashMap::new();
        let live = HashMap::new();

        (fixed, live)
    }
}
