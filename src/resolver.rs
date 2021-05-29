use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};

use swc::{Compiler, config::Options};

use swc_common::FileName;
use swc_bundler::{Load, Resolve};
use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};

pub struct Resolver {
    compiler: Arc<Compiler>,
    options: Options,
    resolver: Box<dyn Resolve>,
    loader: Box<dyn Load>,
}

impl Resolver {
    pub fn new() -> Self {
        let compiler = Arc::new(crate::bundler::get_compiler());
        let options: Options = Default::default();
        Resolver {
            loader: Box::new(
                SwcLoader::new(Arc::clone(&compiler), options.clone())),
            resolver: Box::new(NodeResolver::new()),
            options,
            compiler,
        }
    }

    pub fn resolve<P: AsRef<Path>>(&self, file: P) -> Result<()> {
        log::info!("Resolve {}", file.as_ref().display());
        let file_name = FileName::Real(file.as_ref().to_path_buf());
        let bundler = crate::bundler::get_bundler(
            Arc::clone(&self.compiler),
            self.options.clone(),
            self.compiler.globals(),
            &self.loader,
            &self.resolver);

        let res = bundler
            .load_transformed(&file_name)
            .context("load_transformed failed")?;

        println!("Result {:#?}", res);

        Ok(())
    }
}
