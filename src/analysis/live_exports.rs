//! Helper to analyze live exports from a module.
//!
//! Live exports are exports that have been exported
//! and are assigned later either at the module level
//! or later as part of a function (lazy live export).
//!
//! It is used to indicate that the compiled code should
//! set a proxy trap so changes to the export are propagated
//! out to it's references.
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

/// Live export analysis is done as a separate pass from the
/// export analysis which will be slower but makes the code a lot
/// easier to reason about.
///
/// The export analysis needs to use `visit_module_item()` to
/// detect the exports correctly but this means we would need
/// to branch in many places to detect all the variants for where
/// statements could appear so we detect the statements in a separate
/// visitor pass.
#[derive(Default, Debug)]
pub struct LiveExportAnalysis {
    pub exports: Vec<String>,
    pub live: Vec<String>,
}

impl LiveExportAnalysis {
    pub fn new() -> Self {
        Self {
            exports: Vec::new(),
            live: Default::default(),
        }
    }
}

impl Visit for LiveExportAnalysis {
    fn visit_stmt(&mut self, n: &Stmt, _: &dyn Node) {
        match n {
            // Track assignments for live export map.
            Stmt::Expr(expr) => match &*expr.expr {
                // ++i, i++, --i, i--
                Expr::Update(expr) => match &*expr.arg {
                    Expr::Ident(ident) => {
                        let matched = self
                            .exports
                            .iter()
                            .find(|name| ident.sym.as_ref() == *name);
                        if matched.is_some() {
                            self.live.push(ident.sym.as_ref().to_string());
                        }
                    }
                    _ => {}
                },
                Expr::Assign(expr) => match &expr.left {
                    PatOrExpr::Pat(pat) => match &**pat {
                        Pat::Ident(ident) => {
                            let matched = self
                                .exports
                                .iter()
                                .find(|name| ident.id.sym.as_ref() == *name);
                            if matched.is_some() {
                                self.live
                                    .push(ident.id.sym.as_ref().to_string());
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}
