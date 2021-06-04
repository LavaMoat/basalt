use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};

use spack::resolvers::NodeResolver;
use swc_bundler::Resolve;
use swc_common::{comments::SingleThreadedComments, FileName, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};

const TREE_BAR: &str = "│";
const TREE_BRANCH: &str = "├──";
const TREE_CORNER: &str = "└──";

#[derive(Debug, Default)]
pub struct PrintOptions {
    pub print_tree: bool,
    pub include_file: bool,
}

#[derive(Debug)]
struct PrintBranchState {
    last: bool,
}

#[derive(Debug)]
struct PrintState {
    open: Vec<PrintBranchState>,
    parents: Vec<FileName>,
}

pub(crate) struct Printer {
    resolver: Box<dyn Resolve>,
}

#[derive(Debug)]
pub struct ModuleNode {
    module: Module,
    dependencies: Option<Vec<DependencyDescriptor>>,
    resolved: Vec<(String, FileName)>,
}

impl ModuleNode {
    pub fn analyze(
        &mut self,
        source_map: &SourceMap,
        comments: &SingleThreadedComments,
    ) {
        let deps = analyze_dependencies(&self.module, source_map, comments);
        self.dependencies = if deps.is_empty() { None } else { Some(deps) };
    }

    pub fn resolve(
        &mut self,
        resolver: &Box<dyn Resolve>,
        base: &FileName,
    ) -> Result<()> {
        if let Some(deps) = &self.dependencies {
            for dep in deps {
                let spec = format!("{}", dep.specifier);
                let file_name = resolver.resolve(base, &spec)
                    .context(format!("Failed to resolve module for {}", &spec))?;
                self.resolved.push((spec, file_name));
            }
        }
        Ok(())
    }
}

impl From<Module> for ModuleNode {
    fn from(module: Module) -> Self {
        ModuleNode {
            module,
            dependencies: None,
            resolved: Vec::new(),
        }
    }
}

impl Printer {
    pub fn new() -> Self {
        Printer {
            resolver: Box::new(NodeResolver::new()),
        }
    }

    /// List module imports for an entry point.
    pub fn print<P: AsRef<Path>>(
        &self,
        file: P,
        options: &PrintOptions,
    ) -> Result<()> {
        let mut state = PrintState {
            open: Vec::new(),
            parents: Vec::new(),
        };

        let (_, _, node) = self.parse_file(file.as_ref())?;
        println!("{}", file.as_ref().display());
        self.print_imports(options, node, &mut state)?;

        Ok(())
    }

    /// Parse a file, analyze dependencies and resolve dependency file paths.
    fn parse_file<P: AsRef<Path>>(
        &self,
        file: P,
    ) -> Result<(FileName, Arc<SourceMap>, ModuleNode)> {
        let (file_name, source_map, module) = crate::bundler::load_file(file)?;
        let comments: SingleThreadedComments = Default::default();
        let mut node = ModuleNode::from(module);
        node.analyze(&source_map, &comments);
        node.resolve(&self.resolver, &file_name)?;
        Ok((file_name, source_map, node))
    }

    fn print_imports<'a>(
        &self,
        options: &PrintOptions,
        node: ModuleNode,
        state: &mut PrintState,
    ) -> Result<()> {
        state.open.push(PrintBranchState { last: false });

        for (i, (spec, file_name)) in node.resolved.iter().enumerate() {
            let last = i == (node.resolved.len() - 1);
            let cycles = state.parents.iter().find(|p| p == &file_name);
            state.open.last_mut().unwrap().last = last;

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

                print!("{}", spec);

                if options.include_file {
                    print!(" {}", file_name);
                }

                if let Some(cycle) = cycles {
                    print!(" (∞ -> {})", cycle);
                }

                print!("\n");
            }

            if cycles.is_some() {
                continue;
            }

            match file_name {
                FileName::Real(path) => {
                    // Parse the dependency as a module
                    let (_, _, node) = self.parse_file(path)?;
                    // Recurse for more dependents
                    if !node.resolved.is_empty() {
                        state.parents.push(file_name.clone());
                        self.print_imports(options, node, state)?;
                        state.parents.pop();
                    }
                }
                _ => bail!("Only real paths are supported {:?}", file_name),
            }
        }
        state.open.pop();

        Ok(())
    }
}
