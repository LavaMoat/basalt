//! Analyze imports from builtin modules.
//!
//! Only finds builtin modules that are assigned;
//! side effect imports or calls to require will be ignored
//! under the assumption that built in modules would never
//! be designed for side effects.
//!
use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, VisitAll, VisitAllWith};

use indexmap::IndexSet;

use super::dependencies::is_builtin_module;
use crate::{
    access::Access,
    helpers::{member_expr_words, pattern_words},
};

const REQUIRE: &str = "require";

enum AccessKind {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
enum Local {
    Default(JsWord, Access),
    // Named locals will need to be converted to fully qualified
    // module paths, eg: `readSync` would become the canonical `fs.readSync`
    Named(JsWord, Access),
}

impl Local {
    fn access_mut(&mut self) -> &mut Access {
        match self {
            Self::Default(_, access) => access,
            Self::Named(_, access) => access,
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Builtin {
    source: JsWord,
    locals: Vec<Local>,
}

impl Builtin {
    fn canonical_symbols(&self) -> IndexSet<JsWord> {
        let mut out: IndexSet<JsWord> = Default::default();
        for local in self.locals.iter() {
            match local {
                Local::Default(word, _) => {
                    out.insert(word.clone());
                }
                Local::Named(word, _) => {
                    out.insert(JsWord::from(format!(
                        "{}.{}",
                        self.source, word
                    )));
                }
            }
        }
        out
    }
}

/// Visit a module and generate the set of access
/// to builtin packages.
#[derive(Default)]
pub struct BuiltinAnalysis;

impl BuiltinAnalysis {
    /// Analyze and compute the builtins for a module.
    pub fn analyze(&self, module: &Module) -> IndexSet<JsWord> {
        let mut finder = BuiltinFinder {
            candidates: Default::default(),
        };
        module.visit_all_children_with(&mut finder);

        println!("{:#?}", finder.candidates);

        self.compute(finder.candidates)
    }

    /// Compute the builtins.
    fn compute(&self, candidates: Vec<Builtin>) -> IndexSet<JsWord> {
        let mut out: IndexSet<JsWord> = Default::default();
        for builtin_module in candidates.iter() {
            let symbols = builtin_module.canonical_symbols();
            out = out.union(&symbols).cloned().collect();
        }
        out
    }
}

/// Find the imports and require calls to built in modules.
struct BuiltinFinder {
    candidates: Vec<Builtin>,
}

impl BuiltinFinder {
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

    /// Determine if a word matches a previously located builtin module local
    /// symbol. For member expressions pass the first word in the expression.
    fn is_builtin_match(&mut self, sym: &JsWord) -> Option<&mut Local> {
        for builtin in self.candidates.iter_mut() {
            let mut matched = builtin.locals.iter_mut().find(|local| {
                let word = match local {
                    Local::Default(word, _) => word,
                    Local::Named(word, _) => word,
                };
                word == sym
            });

            if let Some(local) = matched.take() {
                return Some(local);
            }
        }
        None
    }

    fn access_visit_expr(&mut self, n: &Expr, kind: &AccessKind) {
        match n {
            Expr::Ident(n) => {
                if let Some(local) = self.is_builtin_match(&n.sym) {
                    match kind {
                        AccessKind::Read => {
                            local.access_mut().read = true;
                        }
                        AccessKind::Write => {
                            local.access_mut().write = true;
                        }
                        AccessKind::Execute => {
                            local.access_mut().execute = true;
                        }
                    }
                }
            }
            Expr::Member(n) => {
                let members = member_expr_words(n);
                if let Some(word) = members.get(0) {
                    if let Some(local) = self.is_builtin_match(word) {
                        match kind {
                            AccessKind::Read => {
                                local.access_mut().read = true;
                            }
                            AccessKind::Write => {
                                local.access_mut().write = true;
                            }
                            AccessKind::Execute => {
                                local.access_mut().execute = true;
                            }
                        }
                    }
                }
            }
            Expr::Assign(n) => {
                self.access_visit_expr(&n.right, kind);
            }
            Expr::Paren(n) => {
                self.access_visit_expr(&*n.expr, kind);
            }
            Expr::OptChain(n) => {
                self.access_visit_expr(&*n.expr, kind);
            }
            Expr::Unary(n) => {
                self.access_visit_expr(&n.arg, kind);
            }
            Expr::Await(n) => {
                self.access_visit_expr(&n.arg, kind);
            }
            Expr::Yield(n) => {
                if let Some(arg) = &n.arg {
                    self.access_visit_expr(arg, kind);
                }
            }
            Expr::Bin(n) => {
                self.access_visit_expr(&*n.left, kind);
                self.access_visit_expr(&*n.right, kind);
            }
            Expr::Cond(n) => {
                self.access_visit_expr(&*n.test, kind);
                self.access_visit_expr(&*n.cons, kind);
                self.access_visit_expr(&*n.alt, kind);
            }
            Expr::Tpl(n) => {
                for expr in n.exprs.iter() {
                    self.access_visit_expr(&*expr, kind);
                }
            }
            Expr::TaggedTpl(n) => {
                self.access_visit_expr(&*n.tag, kind);
                for expr in n.tpl.exprs.iter() {
                    self.access_visit_expr(&*expr, kind);
                }
            }
            _ => {}
        }
    }
}

impl VisitAll for BuiltinFinder {
    fn visit_import_decl(&mut self, n: &ImportDecl, _: &dyn Node) {
        if is_builtin_module(n.src.value.as_ref()) {
            let mut builtin = Builtin {
                source: n.src.value.clone(),
                locals: Default::default(),
            };

            for spec in n.specifiers.iter() {
                let local = match spec {
                    ImportSpecifier::Default(n) => {
                        Local::Default(n.local.sym.clone(), Default::default())
                    }
                    ImportSpecifier::Named(n) => {
                        Local::Named(n.local.sym.clone(), Default::default())
                    }
                    ImportSpecifier::Namespace(n) => {
                        Local::Default(n.local.sym.clone(), Default::default())
                    }
                };
                if !builtin.locals.contains(&local) {
                    builtin.locals.push(local);
                }
            }
            self.candidates.push(builtin);
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
                    builtin.locals = match &n.name {
                        Pat::Ident(ident) => {
                            vec![Local::Default(
                                ident.id.sym.clone(),
                                Default::default(),
                            )]
                        }
                        _ => {
                            let mut names = Vec::new();
                            pattern_words(&n.name, &mut names);
                            names
                                .into_iter()
                                .cloned()
                                .map(|sym| {
                                    Local::Named(sym, Default::default())
                                })
                                .collect()
                        }
                    };
                    self.candidates.push(builtin);
                }
            } else {
                self.access_visit_expr(&*init, &AccessKind::Read);
            }
        }
    }

    fn visit_expr(&mut self, n: &Expr, _: &dyn Node) {
        match n {
            // Write access on left-hand side of an assignment
            Expr::Assign(n) => {
                match &n.left {
                    PatOrExpr::Pat(n) => match &**n {
                        Pat::Ident(n) => {
                            if let Some(local) = self.is_builtin_match(&n.id.sym) {
                                local.access_mut().write = true;
                            }
                        }
                        Pat::Expr(n) => {
                            self.access_visit_expr(n, &AccessKind::Write);
                        },
                        _ => {}
                    },
                    _ => {}
                }
                self.access_visit_expr(&n.right, &AccessKind::Read);
            },
            // Update is a write access
            Expr::Update(n) => {
                self.access_visit_expr(&*n.arg, &AccessKind::Write);
            },
            // Execute access is a function call
            Expr::Call(n) => {
                match &n.callee {
                    ExprOrSuper::Expr(n) => {
                        self.access_visit_expr(n, &AccessKind::Execute);
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }
}
