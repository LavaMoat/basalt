//! Analyze the lexical scopes for a module and generate a tree
//! containing the local symbols and a list of identities which are
//! symbol references that maybe global variables.

use swc_atoms::JsWord;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

use indexmap::IndexSet;

use crate::helpers::{pattern_words, var_symbol_words};

/// Lexical scope.
#[derive(Debug, Default)]
pub struct Scope {
    /// Scopes contained by this scope.
    scopes: Vec<Scope>,
    /// Identifiers local to this scope.
    locals: IndexSet<JsWord>,
    /// Identifiers that are references.
    ///
    /// These could be local or global symbols and we need
    /// to walk all parent scopes to detect if a symbol should
    /// be considered global.
    idents: IndexSet<JsWord>,
}

impl Scope {
    fn new(locals: Option<IndexSet<JsWord>>) -> Self {
        Self {
            scopes: Default::default(),
            locals: locals.unwrap_or(Default::default()),
            idents: Default::default(),
        }
    }
}

/// Analyze the scopes for a module.
#[derive(Debug)]
pub struct ScopeAnalysis {
    root: Scope,
}

impl ScopeAnalysis {
    /// Create a scope analysis.
    pub fn new() -> Self {
        Self {
            root: Scope::new(None),
        }
    }

    /// Compute the global variables.
    pub fn globals(&self) -> IndexSet<JsWord> {
        let mut global_symbols: IndexSet<JsWord> = Default::default();
        self.compute_globals(&self.root, &mut global_symbols, &mut vec![]);
        global_symbols
    }

    fn compute_globals<'a>(
        &self,
        scope: &'a Scope,
        global_symbols: &mut IndexSet<JsWord>,
        locals_stack: &mut Vec<&'a IndexSet<JsWord>>,
    ) {
        locals_stack.push(&scope.locals);

        let mut combined_locals: IndexSet<JsWord> = Default::default();
        for locals in locals_stack.iter() {
            combined_locals = combined_locals.union(locals).cloned().collect();
        }

        let mut diff: IndexSet<JsWord> =
            scope.idents.difference(&combined_locals).cloned().collect();
        for sym in diff.drain(..) {
            global_symbols.insert(sym);
        }

        for scope in scope.scopes.iter() {
            self.compute_globals(scope, global_symbols, locals_stack);
        }

        locals_stack.pop();
    }
}

impl Visit for ScopeAnalysis {
    fn visit_module_item(&mut self, n: &ModuleItem, _: &dyn Node) {
        let scope = &mut self.root;
        match n {
            ModuleItem::ModuleDecl(decl) => match decl {
                ModuleDecl::Import(import) => {
                    for spec in import.specifiers.iter() {
                        let ident = match spec {
                            ImportSpecifier::Named(n) => &n.local.sym,
                            ImportSpecifier::Default(n) => &n.local.sym,
                            ImportSpecifier::Namespace(n) => &n.local.sym,
                        };
                        scope.locals.insert(ident.clone());
                    }
                }
                _ => {}
            },
            ModuleItem::Stmt(stmt) => visit_stmt(stmt, scope, None),
        }
    }
}

fn visit_stmt(n: &Stmt, scope: &mut Scope, locals: Option<IndexSet<JsWord>>) {
    match n {
        Stmt::Decl(decl) => {
            let result = match decl {
                Decl::Fn(n) => Some(vec![&n.ident.sym]),
                Decl::Class(n) => Some(vec![&n.ident.sym]),
                Decl::Var(n) => {
                    let word_list = var_symbol_words(n);
                    let mut out = Vec::new();
                    for (decl, words) in word_list.iter() {
                        for word in words {
                            out.push(*word);
                        }

                        // Recurse on variable declarations with initializers
                        if let Some(ref init) = decl.init {
                            let expr_stmt = Stmt::Expr(ExprStmt {
                                span: DUMMY_SP,
                                expr: init.clone(),
                            });
                            visit_stmt(&expr_stmt, scope, None);
                        }
                    }
                    Some(out)
                }
                _ => None,
            };
            if let Some(result) = result {
                for ident in result.into_iter() {
                    scope.locals.insert(ident.clone());
                }
            }

            match decl {
                Decl::Fn(n) => {
                    let mut func_param_names: IndexSet<JsWord> =
                        Default::default();

                    // Capture function parameters as locals
                    for param in n.function.params.iter() {
                        let mut names = Vec::new();
                        pattern_words(&param.pat, &mut names);

                        let param_names: IndexSet<_> =
                            names.into_iter().map(|n| n.clone()).collect();

                        func_param_names = func_param_names
                            .union(&param_names)
                            .cloned()
                            .collect();
                    }

                    if let Some(ref body) = n.function.body {
                        let block_stmt = Stmt::Block(body.clone());
                        visit_stmt(&block_stmt, scope, Some(func_param_names));
                    }
                }
                _ => {}
            }
        }
        Stmt::With(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::While(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::DoWhile(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::For(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::ForIn(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::ForOf(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::Labeled(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::If(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.cons, &mut next_scope, None);
            scope.scopes.push(next_scope);

            if let Some(ref alt) = n.alt {
                let mut next_scope = Scope::new(None);
                visit_stmt(&*alt, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
        }
        Stmt::Try(n) => {
            let block_stmt = Stmt::Block(n.block.clone());
            let mut next_scope = Scope::new(None);
            visit_stmt(&block_stmt, &mut next_scope, None);
            scope.scopes.push(next_scope);

            if let Some(ref catch_clause) = n.handler {
                let block_stmt = Stmt::Block(catch_clause.body.clone());
                let mut next_scope = Scope::new(None);
                visit_stmt(&block_stmt, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }

            if let Some(ref finalizer) = n.finalizer {
                let block_stmt = Stmt::Block(finalizer.clone());
                let mut next_scope = Scope::new(None);
                visit_stmt(&block_stmt, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
        }
        Stmt::Switch(n) => {
            for case in n.cases.iter() {
                for stmt in case.cons.iter() {
                    let mut next_scope = Scope::new(None);
                    visit_stmt(stmt, &mut next_scope, None);
                    scope.scopes.push(next_scope);
                }
            }
        }
        Stmt::Block(n) => {
            let mut next_scope = Scope::new(locals);
            for stmt in n.stmts.iter() {
                visit_stmt(stmt, &mut next_scope, None);
            }
            scope.scopes.push(next_scope);
        }
        // Find symbol references which is the list of candidates
        // that may be global (or local) variable references.
        Stmt::Expr(n) => visit_expr(&*n.expr, scope),
        _ => {}
    }
}

fn visit_expr(n: &Expr, scope: &mut Scope) {
    match n {
        Expr::Ident(n) => {
            scope.idents.insert(n.sym.clone());
        }
        Expr::Paren(n) => {
            visit_expr(&n.expr, scope);
        }
        Expr::Yield(n) => {
            if let Some(ref arg) = n.arg {
                visit_expr(arg, scope);
            }
        }
        Expr::Await(n) => {
            visit_expr(&n.arg, scope);
        }
        Expr::Arrow(n) => {
            let mut func_param_names: IndexSet<JsWord> = Default::default();

            // Capture arrow function parameters as locals
            for pat in n.params.iter() {
                let mut names = Vec::new();
                pattern_words(pat, &mut names);
                let param_names: IndexSet<_> =
                    names.into_iter().map(|n| n.clone()).collect();
                func_param_names =
                    func_param_names.union(&param_names).cloned().collect();
            }

            match &n.body {
                BlockStmtOrExpr::BlockStmt(block) => {
                    let block_stmt = Stmt::Block(block.clone());
                    visit_stmt(&block_stmt, scope, Some(func_param_names));
                }
                BlockStmtOrExpr::Expr(expr) => {
                    let expr_stmt = Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: expr.clone(),
                    });
                    visit_stmt(&expr_stmt, scope, Some(func_param_names));
                }
            }
        }
        Expr::Call(n) => match &n.callee {
            ExprOrSuper::Expr(expr) => {
                visit_expr(expr, scope);
            }
            _ => {}
        },
        Expr::Update(n) => {
            visit_expr(&n.arg, scope);
        }
        Expr::Assign(assign) => {
            match &assign.left {
                PatOrExpr::Expr(expr) => {
                    visit_expr(expr, scope);
                }
                PatOrExpr::Pat(pat) => match &**pat {
                    Pat::Ident(ident) => {
                        scope.idents.insert(ident.id.sym.clone());
                    }
                    _ => {}
                },
            }
            visit_expr(&*assign.right, scope);
        }
        Expr::Member(n) => {
            match &n.obj {
                ExprOrSuper::Expr(expr) => {
                    visit_expr(expr, scope);
                }
                _ => {}
            }
            visit_expr(&*n.prop, scope);
        }
        Expr::New(n) => {
            visit_expr(&*n.callee, scope);
        }
        _ => {}
    }
}
