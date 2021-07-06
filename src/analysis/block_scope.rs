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
            root: Scope {
                kind: ScopeKind::Module,
                scopes: Default::default(),
                locals: Default::default(),
            },
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
            ModuleItem::Stmt(stmt) => {
                match stmt {
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
                                for (decl, words) in word_list.iter() {
                                    for word in words {
                                        out.push((*word, LocalSymbol::VarDecl));
                                    }
                                }
                                Some(out)
                            }
                            _ => None
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
                    _ => {}
                }
            }
        }
    }
}
