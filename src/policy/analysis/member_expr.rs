//! Walk a member expression.
//!
//! The AST stores member expressions with the left node as the
//! deepest leaf of the tree and we tend to operate left to right
//! for analysis tasks.
use swc_atoms::JsWord;
use swc_ecma_ast::{CallExpr, Expr, ExprOrSuper, Lit, MemberExpr};

const REQUIRE: &str = "require";

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

/// Detect an expression that is a call to `require()`.
///
/// The call must be a simple call expression (single string argument).
///
/// The first entry in the returned tuple is the argument passed to
/// the function call, for deep dot access (eg: `require('buffer').Buffer`)
/// the the first symbol in the dot access (eg: `Buffer`) is the second
/// entry in the tuple.
pub fn is_require_expr<'a>(
    n: &'a Expr,
) -> Option<(&'a JsWord, Option<&'a JsWord>)> {
    is_call_module(n, REQUIRE)
}

fn is_call_module<'a>(
    n: &'a Expr,
    fn_name: &str,
) -> Option<(&'a JsWord, Option<&'a JsWord>)> {
    match n {
        Expr::Call(call) => {
            return is_simple_call(call, fn_name).map(|o| (o, None));
        }
        Expr::Member(n) => {
            let mut expressions = Vec::new();
            walk(n, &mut expressions);

            // `require('buffer').Buffer`
            if let Some(Expr::Call(call)) = expressions.get(0) {
                let prop_name =
                    if let Some(Expr::Ident(id)) = expressions.get(1) {
                        Some(&id.sym)
                    } else {
                        None
                    };
                return is_simple_call(call, fn_name).map(|o| (o, prop_name));
            }
        }
        _ => {}
    }
    None
}

/// Detect an expression that is a call to a function.
///
/// The call must have a single argument and the argument
/// must be a string literal.
fn is_simple_call<'a>(call: &'a CallExpr, fn_name: &str) -> Option<&'a JsWord> {
    if call.args.len() == 1 {
        if let ExprOrSuper::Expr(n) = &call.callee {
            if let Expr::Ident(id) = &**n {
                if id.sym.as_ref() == fn_name {
                    let arg = call.args.get(0).unwrap();
                    if let Expr::Lit(lit) = &*arg.expr {
                        if let Lit::Str(s) = lit {
                            return Some(&s.value);
                        }
                    }
                }
            }
        }
    }
    None
}
