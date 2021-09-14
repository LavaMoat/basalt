use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use serde::Serialize;

use swc_common::{FileName, SourceMap, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_loader::resolve::Resolve;

use crate::{
    helpers::normalize_specifier,
    module::{
        dependencies::is_dependent_module,
        node::{
            cached_modules, parse_module, VisitedDependency, VisitedModule,
        },
    },
};

use super::serializer::Serializer;

const ROOT_PACKAGE: &str = "<root>";

#[derive(Debug, Serialize)]
pub struct ModuleOptions {
    pub package: String,
}

pub(super) fn load_modules<P: AsRef<Path>>(
    file: P,
    source_map: Arc<SourceMap>,
    resolver: &Box<dyn Resolve>,
) -> Result<Expr> {
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
                    normalize_specifier(dep.spec)
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

    transform_modules(list)
}

fn transform_modules(
    modules: Vec<(String, Arc<VisitedModule>)>,
) -> Result<Expr> {
    let mut serializer = Serializer {};

    let mut arr = ArrayLit {
        span: DUMMY_SP,
        elems: vec![],
    };

    //let mut out = Vec::new();
    for (spec, item) in modules {
        match &*item {
            VisitedModule::Module(_, module)
            | VisitedModule::Json(_, module) => {
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

                //let mut entry = ModuleEntry::from((module, dependencies));
                //entry.options.package = spec;

                let mut item = ArrayLit {
                    span: DUMMY_SP,
                    elems: vec![],
                };

                // Module id
                let id = module.id.serialize(&mut serializer)?;
                item.elems.push(Some(ExprOrSpread {
                    spread: None,
                    expr: id.into_boxed_expr(),
                }));

                // Dependencies map
                let deps = dependencies.serialize(&mut serializer)?;
                item.elems.push(Some(ExprOrSpread {
                    spread: None,
                    expr: deps.into_boxed_expr(),
                }));

                // TODO: generate init function

                // Package options
                let opts = ModuleOptions { package: spec };
                let opts = opts.serialize(&mut serializer)?;
                item.elems.push(Some(ExprOrSpread {
                    spread: None,
                    expr: opts.into_boxed_expr(),
                }));

                // Add to the list of all modules
                arr.elems.push(Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Array(item)),
                }));
            }
            _ => {}
        }
    }

    Ok(Expr::Array(arr))
}
