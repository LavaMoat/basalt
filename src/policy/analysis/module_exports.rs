//! Helper for detecting module exports.
use swc_ecma_ast::*;

const MODULE: &str = "module";
const EXPORTS: &str = "exports";

/// Determine if an expression refers to CJS module exports.
pub fn is_module_exports(n: &PatOrExpr) -> bool {
    match n {
        PatOrExpr::Pat(pat) => match &**pat {
            Pat::Expr(expr) => {
                match &**expr {
                    Expr::Member(n) => {
                        if let (ExprOrSuper::Expr(expr), Expr::Ident(prop)) =
                            (&n.obj, &*n.prop)
                        {
                            if let Expr::Ident(obj) = &**expr {
                                return obj.as_ref() == MODULE
                                    && prop.as_ref() == EXPORTS;
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        },
        _ => {}
    }
    false
}
