use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use serde::Serialize;

use swc_common::{FileName, SourceMap, DUMMY_SP};
use swc_ecma_ast::Function;
use swc_ecma_loader::resolve::Resolve;

use crate::module::{
    dependencies::is_dependent_module,
    node::{
        cached_modules, parse_module, ModuleNode, VisitedDependency, VisitedModule,
    }
};

const ROOT_PACKAGE: &str = "<root>";

#[derive(Debug, Serialize)]
pub struct ModuleEntry {
    /// The module id.
    id: u32,
    /// The dependencies mapped from specifier to module id.
    dependencies: HashMap<String, u32>,
    /// The module initialization function.
    init_fn: Function,
    /// The module options
    options: ModuleOptions,
}

impl From<(&ModuleNode, HashMap<String, u32>)> for ModuleEntry {
    fn from(item: (&ModuleNode, HashMap<String, u32>)) -> Self {
        let (node, dependencies) = item;
        Self {
            id: node.id,
            dependencies,
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
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ModuleOptions {
    pub package: String,
}

pub(super) fn load_modules<P: AsRef<Path>>(
    file: P,
    source_map: Arc<SourceMap>,
    resolver: &Box<dyn Resolve>,
) -> Result<Vec<ModuleEntry>> {
    let mut list = Vec::new();
    let module =
        parse_module(file.as_ref(), resolver, Arc::clone(&source_map))?;

    // Add the root entry point module
    list.push((ROOT_PACKAGE.to_string(), Arc::clone(&module)));

    // Visit the module graph and collect the module nodes
    let mut visitor = |dep: VisitedDependency| {
        if let FileName::Real(path) = &dep.file_name {
            let cached = cached_modules();
            if let Some(item) = cached.get(path) {
                let module = item.value();
                let spec = if is_dependent_module(&dep.spec) {
                    dep.spec.to_string()
                } else {
                    ROOT_PACKAGE.to_string()
                };
                list.push((spec, Arc::clone(module)));
            }
        }
        Ok(())
    };

    if let VisitedModule::Module(_, node) = &*module {
        node.visit(source_map, &mut visitor)?;
    }

    Ok(transform_modules(list))
}

fn transform_modules(modules: Vec<(String, Arc<VisitedModule>)>) -> Vec<ModuleEntry> {
    let mut out = Vec::new();
    for (spec, item) in modules {
        match &*item {
            VisitedModule::Module(_, module) | VisitedModule::Json(_, module) => {
                let dependencies: HashMap<String, u32> = module
                    .resolved
                    .iter()
                    .map(|(spec, file_name)| {
                        let id: Option<u32> =
                            if let FileName::Real(path) = &file_name {
                                let cached = cached_modules();
                                if let Some(item) = cached.get(path) {
                                    let module = item.value();
                                    match &**module {
                                        VisitedModule::Module(_, module)
                                        | VisitedModule::Json(_, module) => {
                                            Some(module.id)
                                        }
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                        return (spec.to_string(), id);
                    })
                    .filter(|(_, id)| id.is_some())
                    .map(|(spec, id)| (spec, id.unwrap()))
                    .collect();

                let mut entry = ModuleEntry::from((module, dependencies));
                entry.options.package = spec;
                //println!("{:#?}", entry);

                // TODO: generate init function

                out.push(entry);
            }
            _ => { /* Do not process builtins */ }
        }
    }
    out
}
