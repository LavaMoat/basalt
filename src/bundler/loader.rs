use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use swc_common::{FileName, SourceMap};
use swc_ecma_ast::Function;
use swc_ecma_loader::resolve::Resolve;

use crate::module::node::{
    cached_modules, parse_module, ModuleNode, VisitedDependency, VisitedModule,
};

pub struct ModuleItem {
    /// The module id.
    pub id: usize,
    /// The dependencies mapped from specifier to module id.
    pub dependencies: HashMap<String, usize>,
    /// The module initialization function.
    pub init_fn: Function,
    /// The module options
    pub options: ModuleOptions,
}

pub struct ModuleOptions {
    pub package: String,
}

pub(super) fn load_modules<P: AsRef<Path>>(
    file: P,
    source_map: Arc<SourceMap>,
    resolver: &Box<dyn Resolve>,
) -> Result<Vec<Arc<VisitedModule>>> {
    let mut list = Vec::new();
    let module = parse_module(file.as_ref(), resolver)?;

    // Add the root entry point module
    list.push(Arc::clone(&module));

    // Visit the module graph and collect the module nodes
    let mut visitor = |dep: VisitedDependency| {
        if let FileName::Real(path) = &dep.file_name {
            let cached = cached_modules();
            if let Some(item) = cached.get(path) {
                let module = item.value();
                list.push(Arc::clone(module));
            }
        }
        Ok(())
    };

    if let VisitedModule::Module(_, _, node) = &*module {
        node.visit(&mut visitor)?;
    }

    // TODO: transform the list into ModuleItem

    Ok(list)
}
