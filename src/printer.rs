use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};
use swc::{config::Options, Compiler};
use swc_bundler::{Bundler, Load, Resolve, TransformedModule};
use swc_common::FileName;

const TREE_BAR: &str = "│";
const TREE_BRANCH: &str = "├──";
const TREE_CORNER: &str = "└──";

#[derive(Debug)]
struct TreeIteratorState {
    last: bool,
}

#[derive(Debug)]
struct PrintState {
    pub open: Vec<TreeIteratorState>,
}

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
        let file_name = FileName::Real(file.as_ref().to_path_buf());
        let bundler = crate::bundler::get_bundler(
            Arc::clone(&self.compiler),
            self.compiler.globals(),
            &self.loader,
            &self.resolver,
        );

        log::info!("Transform {}", file.as_ref().display());

        let res = bundler
            .load_transformed(&file_name)
            .context("load_transformed failed")?;

        println!("{}", file.as_ref().display());
        let mut state = PrintState {open: Vec::new()};
        self.print_imports(res, &bundler, &mut state)?;
        Ok(())
    }

    fn print_imports<'a>(
        &self,
        module: Option<TransformedModule>,
        bundler: &Bundler<'a, &'a Box<dyn Load>, &'a Box<dyn Resolve>>,
        state: &mut PrintState,
    ) -> Result<()> {

        if let Some(ref transformed) = module {
            state.open.push(TreeIteratorState {last: false});
            for (i, import) in transformed.imports.specifiers.iter().enumerate() {
                let last = i == (transformed.imports.specifiers.len() - 1);
                state.open.last_mut().unwrap().last = last;
                let source = &import.0;
                let module_id = source.module_id;
                let module = bundler
                    .scope
                    .get_module(module_id)
                    .ok_or_else(|| anyhow!("Failed to lookup module for {}", module_id))
                    .unwrap();

                let mark = if last { TREE_CORNER } else { TREE_BRANCH };

                for (j, iter_state) in state.open.iter().enumerate() {
                    let end = j == (state.open.len() - 1);
                    if !end {
                        if !iter_state.last {
                            print!("{}   ", TREE_BAR);
                        } else {
                            print!("    ");
                        }
                    } else {
                        print!("{} ", mark);
                    }
                }

                println!("{}", source.src.value);

                if !module.imports.specifiers.is_empty() {
                    self.print_imports(Some(module), bundler, state)?;
                }
            }
            state.open.pop();
        }
        Ok(())
    }
}
