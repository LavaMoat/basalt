//! Helper to parse all modules in a dependency graph for performance timing purposes.
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use swc_common::SourceMap;
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeModulesResolver};

use crate::module::node::{
    cached_modules, parse_file, VisitedDependency, VisitedModule,
};

/// Parse all the modules in a dependency graph.
pub fn parse<P: AsRef<Path>>(file: P) -> Result<(usize, usize)> {
    let resolver: Box<dyn Resolve> = Box::new(NodeModulesResolver::default());
    let source_map: Arc<SourceMap> = Arc::new(Default::default());
    let module = parse_file(file.as_ref(), &resolver, Arc::clone(&source_map))?;

    let node = match &*module {
        VisitedModule::Module(_, node) => Some(node),
        VisitedModule::Json(_, node) => Some(node),
        VisitedModule::Builtin(_) => None,
    };

    let mut visited_count = 0;

    // Visitor is a noop
    let mut visitor = |_dep: VisitedDependency| {
        visited_count += 1;
        Ok(())
    };

    if let Some(node) = node {
        node.visit(source_map, &mut visitor)?;
    }

    // WTF: Visited 29146348 modules!
    //eprintln!("Visited {} modules!", visited_count);

    Ok((cached_modules().len(), visited_count))
}
