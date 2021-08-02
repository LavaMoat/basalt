//! Helper to parse all modules in a dependency graph for performance timing purposes.
use std::path::Path;

use anyhow::Result;

use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeResolver};

use crate::module::node::{
    cached_modules, parse_file, VisitedDependency, VisitedModule,
};

/// Parse all the modules in a dependency graph.
pub fn parse<P: AsRef<Path>>(file: P) -> Result<(usize, usize)> {
    let resolver: Box<dyn Resolve> = Box::new(NodeResolver::default());
    let module = parse_file(file.as_ref(), &resolver)?;

    let node = match &*module {
        VisitedModule::Module(_, _, node) => Some(node),
        VisitedModule::Json(_) => None,
        VisitedModule::Builtin(_) => None,
    };

    let mut visited_count = 0;

    // Visitor is a noop
    let mut visitor = |_dep: VisitedDependency| {
        visited_count += 1;
        Ok(())
    };

    if let Some(node) = node {
        node.visit(&mut visitor)?;
    }

    // WTF: Visited 29146348 modules!
    //eprintln!("Visited {} modules!", visited_count);

    Ok((cached_modules().len(), visited_count))
}
