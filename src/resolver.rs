use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

use swc::{config::Options, Compiler};
use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};
use swc_bundler::{Load, ModuleId, Resolve, TransformedModule};
use swc_common::FileName;

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
            loader: Box::new(SwcLoader::new(Arc::clone(&compiler), options.clone())),
            resolver: Box::new(NodeResolver::new()),
            options,
            compiler,
        }
    }

    /// List module imports for an entry point.
    pub fn list<P: AsRef<Path>>(&self, file: P) -> Result<()> {
        log::info!("--- {} (entry) ---", file.as_ref().display());
        let file_name = FileName::Real(file.as_ref().to_path_buf());
        let bundler = crate::bundler::get_bundler(
            Arc::clone(&self.compiler),
            self.compiler.globals(),
            &self.loader,
            &self.resolver,
        );

        let res = bundler
            .load_transformed(&file_name)
            .context("load_transformed failed")?;
        self.print_imports(res, |id| bundler.scope.get_module(id))?;
        Ok(())
    }

    fn print_imports<F>(&self, module: Option<TransformedModule>, lookup: F) -> Result<()>
    where
        F: Fn(ModuleId) -> Option<TransformedModule>,
    {
        if let Some(ref transformed) = module {
            for import in transformed.imports.specifiers.iter() {
                let source = &import.0;
                let module_id = source.module_id;
                let module = lookup(module_id)
                    .ok_or_else(|| anyhow!("Failed to lookup module for {}", module_id))
                    .unwrap();
                log::info!("{} -> {}", source.src.value, module.fm.name);

                //self.print_imports(Some(module), lookup)?;

                //println!("Got import...{:#?}", source);
                //println!("Got module...{:#?}", module);
            }
        }
        Ok(())
    }
}
