//! Analyze the lexical scopes for a module and generate blocks
//! containing the local symbols.

use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

use indexmap::{IndexMap, IndexSet};

use crate::helpers::{var_symbol_words, pattern_words};

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
    fn new() -> Self {
        Self {
            scopes: Default::default(),
            locals: Default::default(),
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
            root: Scope::new(),
        }
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
                            ImportSpecifier::Named(n) => {
                                &n.local.sym
                            }
                            ImportSpecifier::Default(n) => {
                                &n.local.sym
                            }
                            ImportSpecifier::Namespace(n) => {
                                &n.local.sym
                            }
                        };
                        scope.locals.insert(ident.clone());
                    }
                }
                _ => {}
            },
            ModuleItem::Stmt(stmt) => visit_stmt(stmt, scope),
        }
    }
}

fn visit_stmt(n: &Stmt, scope: &mut Scope) {
    match n {
        Stmt::Decl(decl) => {
            let result = match decl {
                Decl::Fn(n) => {
                    Some(vec![&n.ident.sym])
                }
                Decl::Class(n) => {
                    Some(vec![&n.ident.sym])
                }
                Decl::Var(n) => {
                    let word_list = var_symbol_words(n);
                    let mut out = Vec::new();
                    for (_, words) in word_list.iter() {
                        for word in words {
                            out.push(*word);
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
                    let mut next_scope = Scope::new();

                    // Function name is considered local
                    next_scope.locals.insert(n.ident.sym.clone());

                    // Capture function parameters as locals
                    for param in n.function.params.iter() {
                        let mut names = Vec::new();
                        pattern_words(&param.pat, &mut names);
                        for name in names.drain(..) {
                            next_scope.locals.insert(name.clone());
                        }
                    }

                    if let Some(ref body) = n.function.body {
                        let block_stmt = Stmt::Block(body.clone());
                        visit_stmt(&block_stmt, &mut next_scope);
                    }

                    scope.scopes.push(next_scope);
                }
                _ => {}
            }
        }
        Stmt::With(n) => {
            let mut next_scope = Scope::new();
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::While(n) => {
            let mut next_scope = Scope::new();
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::DoWhile(n) => {
            let mut next_scope = Scope::new();
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::For(n) => {
            let mut next_scope = Scope::new();
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::ForIn(n) => {
            let mut next_scope = Scope::new();
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::ForOf(n) => {
            let mut next_scope = Scope::new();
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::Labeled(n) => {
            let mut next_scope = Scope::new();
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::If(n) => {
            let mut next_scope = Scope::new();
            visit_stmt(&*n.cons, &mut next_scope);
            scope.scopes.push(next_scope);

            if let Some(ref alt) = n.alt {
                let mut next_scope = Scope::new();
                visit_stmt(&*alt, &mut next_scope);
                scope.scopes.push(next_scope);
            }

        }
        Stmt::Try(n) => {
            let block_stmt = Stmt::Block(n.block.clone());
            let mut next_scope = Scope::new();
            visit_stmt(&block_stmt, &mut next_scope);
            scope.scopes.push(next_scope);

            if let Some(ref catch_clause) = n.handler {
                let block_stmt = Stmt::Block(catch_clause.body.clone());
                let mut next_scope = Scope::new();
                visit_stmt(&block_stmt, &mut next_scope);
                scope.scopes.push(next_scope);
            }

            if let Some(ref finalizer) = n.finalizer {
                let block_stmt = Stmt::Block(finalizer.clone());
                let mut next_scope = Scope::new();
                visit_stmt(&block_stmt, &mut next_scope);
                scope.scopes.push(next_scope);
            }

        }
        Stmt::Switch(n) => {
            for case in n.cases.iter() {
                for stmt in case.cons.iter() {
                    let mut next_scope = Scope::new();
                    visit_stmt(stmt, &mut next_scope);
                    scope.scopes.push(next_scope);
                }
            }
        }
        Stmt::Block(n) => {
            let mut next_scope = Scope::new();
            for stmt in n.stmts.iter() {
                visit_stmt(stmt, &mut next_scope);
            }
            scope.scopes.push(next_scope);
        }
        // Find ident references which is the list of candidates
        // that may be global variables.
        Stmt::Expr(n) => match &*n.expr {
            Expr::Update(_) => {
                todo!()
            }
            Expr::Assign(assign) => {
                match &assign.left {
                    PatOrExpr::Expr(expr) => match &**expr {
                        Expr::Ident(ident) => {
                            scope.idents.insert(ident.sym.clone());
                        }
                        _ => {}
                    }
                    PatOrExpr::Pat(pat) => match &**pat {
                        Pat::Ident(ident) => {
                            scope.idents.insert(ident.id.sym.clone());
                        }
                        _ => {}
                    }
                }

                match &*assign.right {
                    Expr::Ident(ident) => {
                        scope.idents.insert(ident.sym.clone());
                    }
                    _ => {}
                }
            }
            Expr::New(_) => {
                todo!()
            }
            _ => {}
        }
        _ => {}
    }
}
