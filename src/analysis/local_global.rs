//! Analyze the local and global symbols for a module.
//!
//! To detect globals we need to know which symbols are local
//! so this analysis stores both locals and globals.
//!
//! Strings are interned so cloning the detected AST nodes
//! should be relatively cheap.
//!
use std::collections::{HashMap, HashSet};

use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, VisitAll};

/// Represents a local symbol in the module.
///
/// We keep a reference to the underlying AST node so that we could
/// use this analysis for more advanced tasks such as linting
/// if we want to.
#[derive(Debug)]
pub enum LocalSymbol {
    /// Represents a named import.
    ImportNamed(ImportNamedSpecifier),
    /// Represents a function declaration.
    FnDecl(FnDecl),
    /// Represents a class declaration.
    ClassDecl(ClassDecl),
}

/// Visit a module and detect local and global symbols.
#[derive(Default, Debug)]
pub struct LocalGlobalAnalysis {
    /// All identifiers in the module grouped by symbol.
    pub idents: HashMap<JsWord, Vec<Ident>>,
    /// Identifiers local to this module.
    pub locals: HashMap<JsWord, Vec<LocalSymbol>>,
}

impl LocalGlobalAnalysis {
    /// Get a set of the symbol identifiers that are not
    /// local to this module.
    pub fn globals(&self) -> HashSet<&JsWord> {
        let idents: HashSet<_> = self.idents.iter().map(|(k, _)| k).collect();
        let locals: HashSet<_> = self.locals.iter().map(|(k, _)| k).collect();
        idents.difference(&locals).map(|k| *k).collect::<HashSet<_>>()
    }
}

impl VisitAll for LocalGlobalAnalysis {

    fn visit_decl(
        &mut self,
        n: &Decl,
        _: &dyn Node,
    ) {
        match n {
            Decl::Fn(func) => {
                let locals = self.locals.entry(func.ident.sym.clone()).or_insert(Vec::new());
                locals.push(LocalSymbol::FnDecl(func.clone()));
            }
            Decl::Class(class) => {
                let locals = self.locals.entry(class.ident.sym.clone()).or_insert(Vec::new());
                locals.push(LocalSymbol::ClassDecl(class.clone()));
            }
            Decl::Var(var) => {
                for decl in var.decls.iter() {

                }
            }
            _ => {/* Ignore typescript declarations */}
        }
    }

    fn visit_import_named_specifier(
        &mut self,
        n: &ImportNamedSpecifier,
        _: &dyn Node,
    ) {
        let locals = self.locals.entry(n.local.sym.clone()).or_insert(Vec::new());
        locals.push(LocalSymbol::ImportNamed(n.clone()));
    }

    fn visit_ident(&mut self, n: &Ident, _: &dyn Node) {
        let idents = self.idents.entry(n.sym.clone()).or_insert(Vec::new());
        idents.push(n.clone());
    }
}
