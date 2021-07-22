//! Analyze imports from builtin modules.
//!
use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, VisitAll};

use indexmap::IndexSet;

use super::dependencies::is_builtin_module;

const REQUIRE: &str = "require";

/// Visit a module and generate the set of access
/// to builtin packages.
pub struct BuiltinAnalysis;

impl BuiltinAnalysis {
    /// Create a builtin analysis.
    pub fn new() -> Self {
        Self {}
    }

    /// Compute the builtins.
    pub fn compute(&self) -> IndexSet<JsWord> {
        Default::default()
    }
}

impl BuiltinAnalysis {
    // Detect an expression that is a call to `require()`.
    //
    // The call must have a single argument and the argument
    // must be a string literal.
    fn is_require_expression<'a>(&self, n: &'a Expr) -> Option<&'a JsWord> {
        if let Expr::Call(call) = n {
            if call.args.len() == 1 {
                if let ExprOrSuper::Expr(n) = &call.callee {
                    if let Expr::Ident(id) = &**n {
                        if id.sym.as_ref() == REQUIRE {
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
        }
        None
    }
}

impl VisitAll for BuiltinAnalysis {

    fn visit_import_decl(&mut self, n: &ImportDecl, _: &dyn Node) {
        if is_builtin_module(n.src.value.as_ref()) {
            println!("Got built in import {:#?}", n.src.value);
            todo!("Handle builtins for ESM import declarations");
        }
    }

    fn visit_var_declarator(&mut self, n: &VarDeclarator, _: &dyn Node) {
        if let Some(init) = &n.init {
            if let Some(require) = self.is_require_expression(init) {
                if is_builtin_module(require.as_ref()) {
                    println!("Got built in require {:#?}", require);
                }
            }
        }
     }
}
