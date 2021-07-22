//! Analyze imports from builtin modules.
//!
use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, VisitAll};

use indexmap::IndexSet;

use super::dependencies::is_builtin_module;

const REQUIRE: &str = "require";

#[derive(Debug, Hash, Eq, PartialEq)]
enum Local {
    Default(JsWord),
    // Named locals will need to be converted to fully qualified
    // module paths, eg: `readSync` would become the canonical `fs.readSync`
    Named(JsWord),
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Builtin {
    source: JsWord,
    locals: Vec<Local>,
}

impl Builtin {
    fn canonical_symbols(&self) -> IndexSet<JsWord> {
        let mut out: IndexSet<JsWord> = Default::default();
        for local in self.locals.iter() {
            match local {
                Local::Default(word) => {
                    out.insert(word.clone());
                },
                Local::Named(word) => {
                    out.insert(JsWord::from(format!("{}.{}", self.source, word)));
                }
            }
        }
        out
    }
}

/// Visit a module and generate the set of access
/// to builtin packages.
pub struct BuiltinAnalysis {
    candidates: IndexSet<Builtin>,
}

impl BuiltinAnalysis {
    /// Create a builtin analysis.
    pub fn new() -> Self {
        Self {
            candidates: Default::default(),
        }
    }

    /// Compute the builtins.
    pub fn compute(&self) -> IndexSet<JsWord> {
        let mut out: IndexSet<JsWord> = Default::default();
        for builtin_module in self.candidates.iter() {
            let symbols = builtin_module.canonical_symbols();
            // TODO: filter symbols to only those used in the module!
            out = out.union(&symbols).cloned().collect();
        }
        out
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
            let mut builtin = Builtin {
                source: n.src.value.clone(),
                locals: Default::default(),
            };

            for spec in n.specifiers.iter() {
                let local = match spec {
                    ImportSpecifier::Default(n) => {
                        Local::Default(n.local.sym.clone())
                    }
                    ImportSpecifier::Named(n) => {
                        Local::Named(n.local.sym.clone())
                    }
                    ImportSpecifier::Namespace(n) => {
                        Local::Default(n.local.sym.clone())
                    }
                };
                if !builtin.locals.contains(&local) {
                    builtin.locals.push(local);
                }
            }

            //println!("{:#?}", builtin);

            self.candidates.insert(builtin);
        }
    }

    fn visit_var_declarator(&mut self, n: &VarDeclarator, _: &dyn Node) {
        if let Some(init) = &n.init {
            if let Some(name) = self.is_require_expression(init) {
                if is_builtin_module(name.as_ref()) {
                    let mut builtin = Builtin {
                        source: name.clone(),
                        locals: Default::default(),
                    };

                    //println!("{:#?}", builtin);

                    self.candidates.insert(builtin);
                }
            }
        }
    }
}
