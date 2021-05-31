use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};
use swc::{config::Options, Compiler};
use swc_bundler::{Bundler, Load, ModuleId, Resolve, TransformedModule};
use swc_common::{FileName/*, SourceMap */};

//use petgraph::graphmap::DiGraphMap;

const TREE_BAR: &str = "│";
const TREE_BRANCH: &str = "├──";
const TREE_CORNER: &str = "└──";

#[derive(Debug, Default)]
pub struct PrintOptions {
    pub print_tree: bool,
    pub print_logs: bool,
    pub include_id: bool,
    pub include_file: bool,
}

#[derive(Debug)]
struct PrintBranchState {
    last: bool,
}

#[derive(Debug)]
struct PrintParent {
    id: ModuleId,
}

#[derive(Debug)]
struct PrintState {
    open: Vec<PrintBranchState>,
    parents: Vec<PrintParent>,
}

pub(crate) struct Printer {
    //source_map: Arc<SourceMap>,
    compiler: Arc<Compiler>,
    resolver: Box<dyn Resolve>,
    loader: Box<dyn Load>,
}

impl Printer {
    pub fn new() -> Self {
        let (_source_map, compiler) = crate::bundler::get_compiler();
        let options: Options = Default::default();
        Printer {
            loader: Box::new(SwcLoader::new(Arc::clone(&compiler), options)),
            resolver: Box::new(NodeResolver::new()),
            //source_map,
            compiler,
        }
    }

    /// List module imports for an entry point.
    pub fn print<P: AsRef<Path>>(
        &self,
        file: P,
        options: &PrintOptions,
    ) -> Result<()> {
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
        let mut state = PrintState {
            open: Vec::new(),
            parents: Vec::new(),
        };
        self.print_imports(options, res, &bundler, &mut state)?;
        Ok(())
    }

    fn print_imports<'a>(
        &self,
        options: &PrintOptions,
        module: Option<TransformedModule>,
        bundler: &Bundler<'a, &'a Box<dyn Load>, &'a Box<dyn Resolve>>,
        state: &mut PrintState,
    ) -> Result<()> {
        if let Some(ref transformed) = module {
            state.open.push(PrintBranchState { last: false });
            for (i, import) in transformed.imports.specifiers.iter().enumerate()
            {
                let last = i == (transformed.imports.specifiers.len() - 1);
                let source = &import.0;
                let module_id = source.module_id;

                let cycles = state.parents.iter().find(|p| p.id == module_id);

                state.open.last_mut().unwrap().last = last;

                let module = bundler
                    .scope
                    .get_module(module_id)
                    .ok_or_else(|| {
                        anyhow!("Failed to lookup module for {}", module_id)
                    })
                    .unwrap();

                if options.print_logs {
                    log::debug!(
                        "Module {} {} (depth: {}) {}",
                        module_id,
                        module.fm.name,
                        state.parents.len(),
                        cycles.is_some()
                    );
                }

                if options.print_tree {
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

                    print!("{}", source.src.value);

                    if options.include_id {
                        print!(" ({})", module_id);
                    }

                    if options.include_file {
                        print!(" {}", module.fm.name);
                    }

                    if let Some(cycle) = cycles {
                        print!(" (∞ -> {})", cycle.id);
                    }

                    print!("\n");
                }

                if cycles.is_some() {
                    continue;
                }

                if !module.imports.specifiers.is_empty() {
                    state.parents.push(PrintParent { id: module_id });
                    if options.print_logs {
                        log::debug!(
                            "Entering child imports {} (depth: {})",
                            module_id,
                            state.parents.len()
                        );

                        for (i, p) in state.parents.iter().enumerate() {
                            print!("{}", " ".repeat(i));
                            println!("{:?}", p.id);
                        }
                    }
                    self.print_imports(options, Some(module), bundler, state)?;
                    state.parents.pop();
                    if options.print_logs {
                        log::debug!(
                            "Exiting child imports {} (depth: {})",
                            module_id,
                            state.parents.len()
                        );
                    }
                }
            }
            state.open.pop();
        }
        Ok(())
    }
}
