use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use swc_common::{SourceMap, FileName};
use swc_ecma_loader::resolve::Resolve;

use crate::module::node::{
    parse_module, ModuleNode, VisitedDependency, VisitedModule, cached_modules,
};

pub(super) struct ModuleList {
    pub(super) modules: Vec<Arc<VisitedModule>>,
}

pub(super) fn load_modules<P: AsRef<Path>>(
    file: P,
    source_map: Arc<SourceMap>,
    resolver: &Box<dyn Resolve>,
) -> Result<ModuleList> {
    let mut list = ModuleList { modules: vec![] };
    let module = parse_module(file.as_ref(), resolver)?;

    // Add the root entry point module
    list.modules.push(Arc::clone(&module));

    // Visit the module graph and collect the module nodes
    let mut visitor = |dep: VisitedDependency| {
        if let FileName::Real(path) = &dep.file_name {
            let cached = cached_modules();
            if let Some(item) = cached.get(path) {
                println!("Registering module {:#?}", path);
                let module = item.value();
                list.modules.push(Arc::clone(module));
            }
        }
        Ok(())
    };

    if let VisitedModule::Module(_, _, node) = &*module {
        node.visit(&mut visitor)?;
    }

    Ok(list)
}
