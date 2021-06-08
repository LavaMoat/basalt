//! Helper to analyze exports from a module.
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
pub struct LiveExportAnalysis<'a> {
    pub exports: Vec<&'a str>,
    pub live: Vec<String>,
}

impl<'a> LiveExportAnalysis<'a> {
    pub fn new(exports: Vec<&'a str>) -> Self {
        Self {
            exports,
            live: Default::default(),
        }
    }
}

impl<'a> Visit for LiveExportAnalysis<'a> {
    fn visit_stmt(
        &mut self,
        n: &Stmt,
        _: &dyn Node
    ) {
        match n {
            // Track assignments for live export map.
            Stmt::Expr(expr) => match &*expr.expr {
                Expr::Assign(expr) => {
                    match &expr.left {
                        PatOrExpr::Pat(pat) => match &**pat {
                            Pat::Ident(ident) => {
                                // Set if we can find an existing export that would
                                // receive the assignment.
                                for name in self.exports.iter() {
                                    if ident.id.sym.as_ref() == *name {
                                        self.live.push(ident.id.sym.as_ref().to_string());
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            _ => {}
        }
    }
}
