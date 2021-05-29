use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};
use swc::{config::Options, Compiler};
use swc_bundler::{Bundler, Load, Resolve, TransformedModule};
use swc_common::FileName;

pub struct Printer {
    compiler: Arc<Compiler>,
    resolver: Box<dyn Resolve>,
    loader: Box<dyn Load>,
}

impl Printer {
    pub fn new() -> Self {
        let compiler = Arc::new(crate::bundler::get_compiler());
        let options: Options = Default::default();
        Printer {
            loader: Box::new(SwcLoader::new(Arc::clone(&compiler), options)),
            resolver: Box::new(NodeResolver::new()),
            compiler,
        }
    }

    /// List module imports for an entry point.
    pub fn print<P: AsRef<Path>>(&self, file: P) -> Result<()> {
        println!("{}", file.as_ref().display());
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
        self.print_imports(res, &bundler, 0)?;
        Ok(())
    }

    fn print_imports<'a>(
        &self,
        module: Option<TransformedModule>,
        bundler: &Bundler<'a, &'a Box<dyn Load>, &'a Box<dyn Resolve>>,
        mut depth: usize,
    ) -> Result<()> {
        if let Some(ref transformed) = module {
            for (i, import) in transformed.imports.specifiers.iter().enumerate() {
                let last = i == (transformed.imports.specifiers.len() - 1);
                let source = &import.0;
                let module_id = source.module_id;
                let module = bundler
                    .scope
                    .get_module(module_id)
                    .ok_or_else(|| anyhow!("Failed to lookup module for {}", module_id))
                    .unwrap();

                //eprintln!("Index {}, last: {}", i, last);

                let indent = " ".repeat(depth * 4);

                //if depth > 0 && !last {
                    //print!("│  ");
                //}

                print!("{}", indent);

                if !last {
                    print!("├── ");
                } else {
                    print!("└── ");
                }

                //println!("{} -> {}", source.src.value, module.fm.name);
                println!("{}", source.src.value);

                if !module.imports.specifiers.is_empty() {
                    //println!("Entering with {}", module.imports.specifiers.len());
                    //println!("{:#?}", module.imports.specifiers);
                    depth += 1;
                    self.print_imports(Some(module), bundler, depth)?;
                    depth -= 1;
                }

                //println!("Got import...{:#?}", source);
                //println!("Got module...{:#?}", module);
            }
        }
        Ok(())
    }
}
