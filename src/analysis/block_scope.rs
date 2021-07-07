//! Analyze the lexical scopes for a module and generate blocks
//! containing the local symbols.

use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

use indexmap::{IndexMap, IndexSet};

use crate::helpers::var_symbol_words;

/// Symbol local to a scope.
#[derive(Debug)]
pub enum LocalSymbol {
    /// Represents a named import.
    ImportNamed,
    /// Represents a default import.
    ImportDefault,
    /// Represents a wildcard import with a local alias name.
    ImportStarAs,
    /// Represents a function declaration.
    FnDecl,
    /// Represents a class declaration.
    ClassDecl,
    /// Represents a variable declaration.
    VarDecl,
}

/// Enumerates the kinds of scopes.
#[derive(Debug)]
pub enum ScopeKind {
    /// Module scope.
    Module,
    /// Class scope.
    Class,
    /// Function scope.
    Function,
    /// Block scope.
    Block,
    /// With scope.
    With,
    /// While scope.
    While,
    /// Do while scope.
    DoWhile,
    /// For scope.
    For,
    /// For in scope.
    ForIn,
    /// For of scope.
    ForOf,
    /// Labeled scope.
    Labeled,
    /// If scope.
    If,
    /// Try scope.
    Try,
    /// Catch scope.
    Catch,
    /// Finally scope.
    Finally,
}

/// Lexical scope.
#[derive(Debug)]
pub struct Scope {
    /// The kind of scope.
    kind: ScopeKind,
    /// Scopes contained by this scope.
    scopes: Vec<Scope>,
    /// Identifiers local to this scope.
    locals: IndexMap<JsWord, Vec<LocalSymbol>>,
    /// Identifiers that are references.
    ///
    /// These could be local or global symbols and we need
    /// to walk all parent scopes to detect if a symbol should
    /// be considered global.
    idents: IndexSet<JsWord>,
}

impl Scope {
    fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
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
            root: Scope::new(ScopeKind::Module),
        }
    }
}

impl ScopeAnalysis {
}

impl Visit for ScopeAnalysis {
    fn visit_module_item(&mut self, n: &ModuleItem, _: &dyn Node) {
        let scope = &mut self.root;
        match n {
            ModuleItem::ModuleDecl(decl) => match decl {
                ModuleDecl::Import(import) => {
                    for spec in import.specifiers.iter() {
                        let (ident, symbol) = match spec {
                            ImportSpecifier::Named(n) => {
                                (&n.local.sym, LocalSymbol::ImportNamed)
                            }
                            ImportSpecifier::Default(n) => {
                                (&n.local.sym, LocalSymbol::ImportDefault)
                            }
                            ImportSpecifier::Namespace(n) => {
                                (&n.local.sym, LocalSymbol::ImportStarAs)
                            }
                        };
                        let locals = scope
                            .locals
                            .entry(ident.clone())
                            .or_insert(Vec::new());
                        locals.push(symbol);
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
                    Some(vec![(&n.ident.sym, LocalSymbol::FnDecl)])
                }
                Decl::Class(n) => {
                    Some(vec![(&n.ident.sym, LocalSymbol::ClassDecl)])
                }
                Decl::Var(n) => {
                    let word_list = var_symbol_words(n);
                    let mut out = Vec::new();
                    for (_, words) in word_list.iter() {
                        for word in words {
                            out.push((*word, LocalSymbol::VarDecl));
                        }
                    }
                    Some(out)
                }
                _ => None,
            };
            if let Some(result) = result {
                for (ident, symbol) in result.into_iter() {
                    let locals = scope
                        .locals
                        .entry(ident.clone())
                        .or_insert(Vec::new());
                    locals.push(symbol);
                }
            }
        }
        Stmt::With(n) => {
            let mut next_scope = Scope::new(ScopeKind::With);
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::While(n) => {
            let mut next_scope = Scope::new(ScopeKind::While);
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::DoWhile(n) => {
            let mut next_scope = Scope::new(ScopeKind::DoWhile);
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::For(n) => {
            let mut next_scope = Scope::new(ScopeKind::For);
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::ForIn(n) => {
            let mut next_scope = Scope::new(ScopeKind::ForIn);
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::ForOf(n) => {
            let mut next_scope = Scope::new(ScopeKind::ForOf);
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::Labeled(n) => {
            let mut next_scope = Scope::new(ScopeKind::Labeled);
            visit_stmt(&*n.body, &mut next_scope);
            scope.scopes.push(next_scope);
        }
        Stmt::If(n) => {
            let mut next_scope = Scope::new(ScopeKind::If);
            visit_stmt(&*n.cons, &mut next_scope);
            scope.scopes.push(next_scope);

            if let Some(ref alt) = n.alt {
                let mut next_scope = Scope::new(ScopeKind::If);
                visit_stmt(&*alt, &mut next_scope);
                scope.scopes.push(next_scope);
            }

        }
        Stmt::Try(n) => {
            let block_stmt = Stmt::Block(n.block.clone());
            let mut next_scope = Scope::new(ScopeKind::Try);
            visit_stmt(&block_stmt, &mut next_scope);
            scope.scopes.push(next_scope);

            if let Some(ref catch_clause) = n.handler {
                let block_stmt = Stmt::Block(catch_clause.body.clone());
                let mut next_scope = Scope::new(ScopeKind::Catch);
                visit_stmt(&block_stmt, &mut next_scope);
                scope.scopes.push(next_scope);
            }

            if let Some(ref finalizer) = n.finalizer {
                let block_stmt = Stmt::Block(finalizer.clone());
                let mut next_scope = Scope::new(ScopeKind::Finally);
                visit_stmt(&block_stmt, &mut next_scope);
                scope.scopes.push(next_scope);
            }

        }
        Stmt::Block(n) => {
            let mut next_scope = Scope::new(ScopeKind::Block);
            for stmt in n.stmts.iter() {
                visit_stmt(stmt, &mut next_scope);
            }
            scope.scopes.push(next_scope);
        }
        _ => {}
    }
}
