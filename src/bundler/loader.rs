use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use swc_common::{FileName, SourceMap, DUMMY_SP};
use swc_ecma_ast::Function;
use swc_ecma_loader::resolve::Resolve;

use crate::module::node::{
    cached_modules, parse_module, ModuleNode, VisitedDependency, VisitedModule,
};

const ROOT_PACKAGE: &str = "<root>";

pub struct ModuleEntry {
    /// The module id.
    pub id: usize,
    /// The dependencies mapped from specifier to module id.
    pub dependencies: HashMap<String, usize>,
    /// The module initialization function.
    pub init_fn: Function,
    /// The module options
    pub options: ModuleOptions,
}

impl From<&ModuleNode> for ModuleEntry {
    fn from(node: &ModuleNode) -> Self {
        Self {
            id: node.id,
            dependencies: HashMap::new(),
            init_fn: Function {
                params: vec![],
                decorators: vec![],
                span: DUMMY_SP,
                body: None,
                is_generator: false,
                is_async: false,
                type_params: None,
                return_type: None,
            },
            options: ModuleOptions {
                package: ROOT_PACKAGE.into(),
            }
        }
    }
}

pub struct ModuleOptions {
    pub package: String,
}

pub(super) fn load_modules<P: AsRef<Path>>(
    file: P,
    source_map: Arc<SourceMap>,
    resolver: &Box<dyn Resolve>,
) -> Result<Vec<ModuleEntry>> {
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

    Ok(transform_modules(list))
}

fn transform_modules(modules: Vec<Arc<VisitedModule>>) -> Vec<ModuleEntry> {
    let mut out = Vec::new();
    for item in modules {
        match &*item {
            VisitedModule::Module(_, _, module) => {
                let entry = ModuleEntry::from(module);
                // TODO: compute dependencies
                // TODO: generate init function
                // TODO: compute package options
                out.push(entry);
            }
            _ => { /* Do not process JSON or builtins */ },
        }
    }
    out
}
