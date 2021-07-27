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

use indexmap::IndexMap;

use super::dependencies::is_builtin_module;
use crate::{
    access::Access,
    helpers::{member_expr_words, pattern_words},
};

const REQUIRE: &str = "require";
const CONSOLE: &str = "console";
const PROCESS: &str = "process";

const PERF_HOOKS: &str = "perf_hooks";
const PERFORMANCE: &str = "performance";

enum AccessKind {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
enum Local {
    Default(JsWord),
    // Named locals will need to be converted to fully qualified
    // module paths, eg: `readSync` would become the canonical `fs.readSync`
    Named(JsWord),
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Builtin {
    source: JsWord,
    locals: Vec<Local>,
}

/// Options for builtin analysis.
#[derive(Debug)]
pub struct BuiltinOptions {
    /// Expose the nodejs global buitin modules (eg: `console` and `process`) automatically.
    node_global_builtins: bool,
}

impl Default for BuiltinOptions {
    fn default() -> Self {
        Self {
            node_global_builtins: true,
        }
    }
}

/// Visit a module and generate the set of access
/// to builtin packages.
#[derive(Default)]
pub struct BuiltinAnalysis {
    options: BuiltinOptions,
}

impl BuiltinAnalysis {
    /// Create a builtin analysis.
    pub fn new(options: BuiltinOptions) -> Self {
        Self { options }
    }

    /// Analyze and compute the builtins for a module.
    pub fn analyze(&self, module: &Module) -> IndexMap<JsWord, Access> {
        let mut finder = BuiltinFinder {
            candidates: Default::default(),
            access: Default::default(),
        };

        if self.options.node_global_builtins {
            finder.candidates.push(Builtin {
                source: JsWord::from(PROCESS),
                locals: vec![Local::Default(JsWord::from(PROCESS))],
            });

            finder.candidates.push(Builtin {
                source: JsWord::from(CONSOLE),
                locals: vec![Local::Default(JsWord::from(CONSOLE))],
            });

            finder.candidates.push(Builtin {
                source: JsWord::from(PERF_HOOKS),
                locals: vec![Local::Default(JsWord::from(PERFORMANCE))],
            });
        }

        module.visit_all_children_with(&mut finder);
        self.compute(finder.access)
    }

    /// Compute the builtins.
    fn compute(
        &self,
        access: IndexMap<Vec<JsWord>, Access>,
    ) -> IndexMap<JsWord, Access> {
        let mut out: IndexMap<JsWord, Access> = Default::default();
        for (words, access) in access {
            let words: Vec<String> =
                words.into_iter().map(|w| w.as_ref().to_string()).collect();
            out.insert(JsWord::from(words.join(".")), access.clone());
        }
        out
    }
}

/// Find the imports and require calls to built in modules.
struct BuiltinFinder {
    candidates: Vec<Builtin>,
    access: IndexMap<Vec<JsWord>, Access>,
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
    fn is_builtin_match(&mut self, sym: &JsWord) -> Option<(&Local, JsWord)> {
        for builtin in self.candidates.iter() {
            let mut matched = builtin.locals.iter().find(|local| {
                let word = match local {
                    Local::Default(word) => word,
                    Local::Named(word) => word,
                };
                word == sym
            });
            if let Some(local) = matched.take() {
                return Some((local, builtin.source.clone()));
            }
        }
        None
    }

    fn access_visit_expr(&mut self, n: &Expr, kind: &AccessKind) {
        match n {
            Expr::Ident(n) => {
                if let Some((local, source)) = self.is_builtin_match(&n.sym) {
                    let words_key = match local {
                        Local::Named(word) => vec![source, word.clone()],
                        Local::Default(word) => vec![word.clone()],
                    };
                    let entry = self
                        .access
                        .entry(words_key)
                        .or_insert(Default::default());
                    match kind {
                        AccessKind::Read => {
                            entry.read = true;
                        }
                        AccessKind::Write => {
                            entry.write = true;
                        }
                        AccessKind::Execute => {
                            entry.execute = true;
                        }
                    }
                }
            }
            Expr::Member(n) => {
                let members = member_expr_words(n);
                if let Some(word) = members.get(0) {
                    if let Some((local, source)) = self.is_builtin_match(word) {
                        let mut words_key: Vec<JsWord> =
                            members.into_iter().cloned().collect();
                        if let Local::Named(_) = local {
                            words_key.insert(0, source);
                        }

                        let entry = self
                            .access
                            .entry(words_key)
                            .or_insert(Default::default());

                        match kind {
                            AccessKind::Read => {
                                entry.read = true;
                            }
                            AccessKind::Write => {
                                entry.write = true;
                            }
                            AccessKind::Execute => {
                                entry.execute = true;
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
                            vec![Local::Default(ident.id.sym.clone())]
                        }
                        _ => {
                            let mut names = Vec::new();
                            pattern_words(&n.name, &mut names);
                            names
                                .into_iter()
                                .cloned()
                                .map(|sym| Local::Named(sym))
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
                            if let Some((local, source)) =
                                self.is_builtin_match(&n.id.sym)
                            {
                                let words_key = match local {
                                    Local::Named(word) => {
                                        vec![source, word.clone()]
                                    }
                                    Local::Default(word) => vec![word.clone()],
                                };
                                let entry = self
                                    .access
                                    .entry(words_key)
                                    .or_insert(Default::default());
                                entry.write = true;
                            }
                        }
                        Pat::Expr(n) => {
                            self.access_visit_expr(n, &AccessKind::Write);
                        }
                        _ => {}
                    },
                    _ => {}
                }
                self.access_visit_expr(&n.right, &AccessKind::Read);
            }
            // Update is a write access
            Expr::Update(n) => {
                self.access_visit_expr(&*n.arg, &AccessKind::Write);
            }
            // Execute access is a function call
            Expr::Call(n) => match &n.callee {
                ExprOrSuper::Expr(n) => {
                    self.access_visit_expr(n, &AccessKind::Execute);
                }
                _ => {}
            },
            _ => {}
        }
    }
}
