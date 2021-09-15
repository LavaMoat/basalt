use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Result};
use serde::Serialize;

use swc_common::{FileName, SourceMap, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_loader::resolve::Resolve;
use swc_ecma_visit::{Node, Visit, VisitWith};

use crate::{
    helpers::{is_module_exports, normalize_specifier, EXPORTS, MODULE},
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
    pub r#type: ModuleKind,
}

#[derive(Debug, Serialize)]
pub enum ModuleKind {
    #[serde(skip)]
    Mixed,
    #[serde(rename = "esm")]
    Esm,
    #[serde(rename = "cjs")]
    Cjs,
}

pub(super) fn load_modules<P: AsRef<Path>>(
    file: P,
    source_map: Arc<SourceMap>,
    resolver: &Box<dyn Resolve>,
) -> Result<(Expr, Vec<u32>)> {
    let mut list = Vec::new();
    let module =
        parse_module(file.as_ref(), resolver, Arc::clone(&source_map))?;

    let root_entry_id = {
        let cached = cached_modules();
        let entry = cached.get(file.as_ref()).unwrap();
        let value = entry.value();
        match &**value {
            VisitedModule::Module(_, node) | VisitedModule::Json(_, node) => {
                node.id
            }
            _ => unreachable!("Main entry point cannot be a builtin"),
        }
    };

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

    Ok((transform_modules(list)?, vec![root_entry_id]))
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

                // Transform to init function
                let (init_fn, kind) = into_module_function(&*module.module)?;
                item.elems.push(Some(ExprOrSpread {
                    spread: None,
                    expr: init_fn,
                }));

                // Package options
                let opts = ModuleOptions {
                    package: spec,
                    r#type: kind,
                };
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

fn into_module_function(module: &Module) -> Result<(Box<Expr>, ModuleKind)> {
    let mut detector = Es6Detector {
        esm: false,
        cjs: false,
    };
    module.visit_children_with(&mut detector);

    let kind = detector.kind();
    match &kind {
        ModuleKind::Esm => Ok((transform_esm(module)?, kind)),
        ModuleKind::Cjs => Ok((transform_cjs(module)?, kind)),
        ModuleKind::Mixed => {
            bail!("ESM and CJS modules may not be combined")
        }
    }
}

fn transform_esm(module: &Module) -> Result<Box<Expr>> {
    let expr = Expr::Lit(Lit::Null(Null { span: DUMMY_SP }));
    Ok(Box::new(expr))
}

fn transform_cjs(module: &Module) -> Result<Box<Expr>> {
    let expr = Expr::Fn(FnExpr {
        ident: None,
        function: Function {
            decorators: vec![],
            params: vec![
                Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: Pat::Ident(BindingIdent {
                        id: Ident {
                            span: DUMMY_SP,
                            sym: MODULE.into(),
                            optional: false,
                        },
                        type_ann: None,
                    }),
                },
                Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: Pat::Ident(BindingIdent {
                        id: Ident {
                            span: DUMMY_SP,
                            sym: EXPORTS.into(),
                            optional: false,
                        },
                        type_ann: None,
                    }),
                },
            ],
            span: DUMMY_SP,
            body: Some(BlockStmt {
                span: DUMMY_SP,
                stmts: module
                    .body
                    .iter()
                    .filter(|item| match item {
                        ModuleItem::Stmt(_) => true,
                        _ => false,
                    })
                    .map(|item| match item {
                        ModuleItem::Stmt(stmt) => stmt.clone(),
                        _ => unreachable!(),
                    })
                    .collect(),
            }),
            is_generator: false,
            is_async: false,
            type_params: None,
            return_type: None,
        },
    });
    Ok(Box::new(expr))
}

struct Es6Detector {
    esm: bool,
    cjs: bool,
}

impl Es6Detector {
    fn kind(&self) -> ModuleKind {
        if self.esm && self.cjs {
            ModuleKind::Mixed
        } else if self.esm {
            ModuleKind::Esm
        } else {
            ModuleKind::Cjs
        }
    }
}

impl Visit for Es6Detector {
    fn visit_module_item(&mut self, n: &ModuleItem, _: &dyn Node) {
        match n {
            ModuleItem::ModuleDecl(n) => match n {
                ModuleDecl::Import(_)
                | ModuleDecl::ExportDecl(_)
                | ModuleDecl::ExportNamed(_)
                | ModuleDecl::ExportDefaultDecl(_)
                | ModuleDecl::ExportDefaultExpr(_)
                | ModuleDecl::ExportAll(_) => {
                    self.esm = true;
                }
                _ => {}
            },
            ModuleItem::Stmt(n) => {
                if let Stmt::Expr(n) = n {
                    if let Expr::Assign(n) = &*n.expr {
                        if is_module_exports(&n.left) {
                            self.cjs = true;
                        }
                    }
                }
            }
        }
    }
}
