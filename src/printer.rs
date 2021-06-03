use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};
use swc::{config::Options, Compiler};
use swc_bundler::{
    Bundler, Load, ModuleId, Resolve,
    TransformedModule,
};
use swc_common::FileName;
use swc_bundler_analysis::specifier::Specifier;
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};
use swc_ecma_ast::Module;

use crate::utils::dedupe_import_specifiers;

const TREE_BAR: &str = "│";
const TREE_BRANCH: &str = "├──";
const TREE_CORNER: &str = "└──";

#[derive(Debug, Default)]
pub struct PrintOptions {
    pub print_tree: bool,
    pub include_id: bool,
    pub include_file: bool,
    pub include_exports: bool,
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
    compiler: Arc<Compiler>,
    resolver: Box<dyn Resolve>,
    loader: Box<dyn Load>,
}

struct ModuleNode {
    module: Module,
    dependencies: Option<DependencyDescriptor>,
}

impl Printer {
    pub fn new() -> Self {
        let (_source_map, compiler) = crate::bundler::get_compiler();
        let options: Options = Default::default();
        Printer {
            loader: Box::new(SwcLoader::new(Arc::clone(&compiler), options)),
            resolver: Box::new(NodeResolver::new()),
            compiler,
        }
    }

    /// List module imports for an entry point.
    pub fn print<P: AsRef<Path>>(
        &self,
        file: P,
        options: &PrintOptions,
    ) -> Result<()> {

        let module = crate::bundler::load_file(file)?;

        /*
        let file_name = FileName::Real(file.as_ref().to_path_buf());
        let bundler = crate::bundler::get_bundler(
            Arc::clone(&self.compiler),
            self.compiler.globals(),
            &self.loader,
            &self.resolver,
        );

        log::info!("Transform {}", file.as_ref().display());

        let res = bundler
            .load_transformed(&file_name, true)
            .context("load_transformed failed")?;

        println!("{}", file.as_ref().display());
        let mut state = PrintState {
            open: Vec::new(),
            parents: Vec::new(),
        };

        self.print_imports(options, res, &bundler, &mut state)?;

        */

        Ok(())
    }

    fn print_specifier(&self, item: &Specifier) {
        match item {
            Specifier::Specific { local, alias } => {
                if let Some(alias) = alias {
                    print!("{:?} as ", alias);
                }
                print!("{:?}", local);
            }
            _ => {}
        }
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
            let mut specifiers = transformed.imports.specifiers.clone();
            dedupe_import_specifiers(&mut specifiers);

            // TODO: ensure this is indented for nested modules
            if options.include_exports {
                for spec in transformed.exports.items.iter() {
                    print!("{} > ", TREE_BRANCH);
                    self.print_specifier(spec);
                    print!("\n");
                }
                for item in transformed.exports.reexports.iter() {
                    print!("{} <> ", TREE_BRANCH);
                    if item.1.is_empty() {
                        print!("* from ");
                    } else {
                        print!("{{");
                        for spec in item.1.iter() {
                            self.print_specifier(spec);
                        }
                        print!("}} from ");
                    }
                    print!("{}", item.0.src.value);
                    //println!("{:#?}", item);
                    print!("\n");
                }
            }

            for (i, import) in specifiers.iter().enumerate() {
                let last = i == (specifiers.len() - 1);
                let source = &import.0;
                let module_id = source.module_id;

                let cycles = state.parents.iter().find(|p| p.id == module_id);

                state.open.last_mut().unwrap().last = last;

                let dep = bundler
                    .scope
                    .get_module(module_id)
                    .ok_or_else(|| {
                        anyhow!("Failed to lookup module for {}", module_id)
                    })
                    .unwrap();

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
                        print!(" {}", dep.fm.name);
                    }

                    if let Some(cycle) = cycles {
                        print!(" (∞ -> {})", cycle.id);
                    }

                    print!("\n");
                }

                if cycles.is_some() {
                    continue;
                }

                let mut dep_specifiers = dep.imports.specifiers.clone();
                dedupe_import_specifiers(&mut dep_specifiers);

                if !dep_specifiers.is_empty() {
                    state.parents.push(PrintParent { id: module_id });
                    self.print_imports(options, Some(dep), bundler, state)?;
                    state.parents.pop();
                }
            }
            state.open.pop();
        }
        Ok(())
    }
}
