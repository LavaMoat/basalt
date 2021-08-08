//! Walk a member expression.
//!
//! The AST stores member expressions with the left node as the
//! deepest leaf of the tree and we tend to operate left to right
//! for analysis tasks.
use swc_atoms::JsWord;
use swc_ecma_ast::*;

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
    let mut words = Vec::new();
    walk_member_expressions(n, &mut words);
    words
}

fn walk_member_expressions<'a>(n: &'a MemberExpr, words: &mut Vec<&'a JsWord>) {
    let mut expressions = Vec::new();
    walk(n, &mut expressions);
    for expr in expressions.iter() {
        match expr {
            Expr::Ident(n) => words.push(&n.sym),
            Expr::Member(n) => walk_member_expressions(n, words),
            Expr::Call(n) => match &n.callee {
                ExprOrSuper::Expr(expr) => match &**expr {
                    Expr::Member(n) => walk_member_expressions(n, words),
                    _ => {}
                }
                _ => {}
            },
            _ => {},
        }
    }

}
