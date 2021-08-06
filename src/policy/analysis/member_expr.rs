//! Walk a member expression.
//!
//! The AST stores member expressions with the left node as the
//! deepest leaf of the tree and we tend to operate left to right
//! for analysis tasks.
use swc_ecma_ast::{Expr, ExprOrSuper, MemberExpr};

/// Walk a member expression left to right.
///
/// If a member expression is computed the property is not visited.
pub fn walk<'a>(n: &'a MemberExpr, expressions: &mut Vec<&'a Expr>) {
    if let ExprOrSuper::Expr(n) = &n.obj {
        match &**n {
            Expr::Member(n) => {
                walk(n, expressions);
            }
            _ => walk_member_expr(n, expressions),
        }
    }

    if n.computed {
        return;
    };

    walk_member_expr(&*n.prop, expressions);
}

fn walk_member_expr<'a>(n: &'a Expr, expressions: &mut Vec<&'a Expr>) {
    match n {
        Expr::Member(n) => {
            walk(n, expressions);
        }
        _ => {
            expressions.push(n);
        }
    }
}
