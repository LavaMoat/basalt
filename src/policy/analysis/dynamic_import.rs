//! Helper functions to detect calls to `require` or `import`.

use swc_atoms::JsWord;
use swc_ecma_ast::*;

use super::member_expr::walk;

const REQUIRE: &str = "require";
const IMPORT: &str = "import";

/// Import that is a function call.
pub struct DynamicCall<'a> {
    /// Function name.
    pub fn_name: &'static str,
    /// Argument passed to the function.
    pub arg: &'a JsWord,
    /// Property name when call is a member expression.
    pub member: Option<&'a JsWord>
}

/// Detect an expression that is a call to `require()`.
///
/// The call must be a simple call expression (single string argument).
pub fn is_require_expr<'a>(
    n: &'a Expr,
) -> Option<DynamicCall<'a>> {
    is_call_module(n, REQUIRE)
}

/// Detect an expression that is a call to `import()`.
///
/// The call must be a simple call expression (single string argument).
pub fn is_import_expr<'a>(
    n: &'a Expr,
) -> Option<DynamicCall<'a>> {
    is_call_module(n, IMPORT)
}

fn is_call_module<'a>(
    n: &'a Expr,
    fn_name: &'static str,
) -> Option<DynamicCall<'a>> {
    match n {
        Expr::Call(call) => {
            return is_simple_call(call, fn_name).map(|arg| {
                DynamicCall { arg, member: None, fn_name }
            });
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
                return is_simple_call(call, fn_name).map(|arg| {
                    DynamicCall { arg, member: prop_name, fn_name }
                });
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
fn is_simple_call<'a>(call: &'a CallExpr, fn_name: &'static str) -> Option<&'a JsWord> {
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
