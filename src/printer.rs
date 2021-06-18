//! Utility to print the module graph as a tree.
use std::path::Path;

use anyhow::Result;
use spack::resolvers::NodeResolver;
use swc_bundler::Resolve;

use crate::module_node::{parse_file, VisitedDependency};

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
        let resolver: Box<dyn Resolve> = Box::new(NodeResolver::new());
        let (_, _, node) = parse_file(file.as_ref(), &resolver)?;
        println!("{}", file.as_ref().display());

        let visitor = |dep: VisitedDependency| {
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
        };
        node.visit(&visitor)?;

        Ok(())
    }
}
