//! Utility to print the module graph as a tree.
use std::path::Path;

use anyhow::Result;

use crate::module::node::{parse_file, VisitedDependency, VisitedModule};

use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeModulesResolver};

const TREE_BAR: &str = "│";
const TREE_BRANCH: &str = "├──";
const TREE_CORNER: &str = "└──";

/// Options to use when printing the module graph.
#[derive(Debug, Default)]
pub struct PrintOptions {
    /// Include file names.
    pub include_file: bool,
}

/// Prints the module graph as a tree.
pub struct Printer;

impl Printer {
    /// Creates a new module graph printer.
    pub fn new() -> Self {
        Printer {}
    }

    /// List module imports for an entry point.
    pub fn print<P: AsRef<Path>>(
        &self,
        file: P,
        options: &PrintOptions,
    ) -> Result<()> {
        let resolver: Box<dyn Resolve> = Box::new(NodeModulesResolver::default());
        let module = parse_file(file.as_ref(), &resolver)?;
        let node = match &*module {
            VisitedModule::Module(_, _, node) => Some(node),
            VisitedModule::Json(_) => None,
            VisitedModule::Builtin(_) => None,
        };
        println!("{}", file.as_ref().display());

        let mut visitor = |dep: VisitedDependency| {
            let mark = if dep.last { TREE_CORNER } else { TREE_BRANCH };
            for (j, iter_state) in dep.state.open.iter().enumerate() {
                let end = j == (dep.state.open.len() - 1);
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

            print!("{}", &dep.spec);

            if options.include_file {
                print!(" {}", dep.file_name);
            }

            if let Some(cycle) = dep.cycles {
                print!(" (∞ -> {})", cycle);
            }

            print!("\n");

            Ok(())
        };

        if let Some(node) = node {
            node.visit(&mut visitor)?;
        }

        Ok(())
    }
}
