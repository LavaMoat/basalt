//! Walk a member expression.
//!
//! The AST stores member expressions with the left node as the
//! deepest leaf of the tree and we tend to operate left to right
//! for analysis tasks.
use swc_ecma_ast::{Expr, ExprOrSuper, MemberExpr};
use swc_atoms::JsWord;

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

/// Collect the words in a member expression.
pub fn member_expr_words(n: &MemberExpr) -> Vec<&JsWord> {
    let mut expressions = Vec::new();
    walk(n, &mut expressions);
    expressions
        .iter()
        .filter_map(|e| match e {
            Expr::Ident(id) => Some(&id.sym),
            _ => None,
        })
        .collect()
}

