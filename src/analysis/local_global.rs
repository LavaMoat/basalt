//! Analyze the local and global symbols for a module.
//!
//! To detect globals we need to know which symbols are local
//! so this analysis stores both locals and globals.
//!
//! Strings are interned so cloning the detected AST nodes
//! should be relatively cheap.
//!
use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, VisitAll};

use indexmap::{IndexMap, IndexSet};

use crate::helpers::var_symbol_words;

/// Represents a local symbol in the module.
///
/// We keep a reference to the underlying AST node so that we could
/// use this analysis for more advanced tasks such as linting
/// if we want to.
#[derive(Debug)]
pub enum LocalSymbol {
    /// Represents a named import.
    ImportNamed(ImportNamedSpecifier),
    /// Represents a wildcard import with a local alias name.
    ImportStarAs(ImportStarAsSpecifier),
    /// Represents a function declaration.
    FnDecl(FnDecl),
    /// Represents a class declaration.
    ClassDecl(ClassDecl),
    /// Represents a variable declaration.
    VarDecl(VarDeclarator),
}

/// Visit a module and detect local and global symbols.
#[derive(Default, Debug)]
pub struct LocalGlobalAnalysis {
    /// All identifiers in the module grouped by symbol.
    idents: IndexMap<JsWord, Vec<Ident>>,
    /// Identifiers local to this module.
    locals: IndexMap<JsWord, Vec<LocalSymbol>>,
}

impl LocalGlobalAnalysis {
    /// Get a set of the symbol identifiers that are not
    /// local to this module.
    pub fn globals(&self) -> IndexSet<&JsWord> {
        let idents: IndexSet<_> = self.idents.iter().map(|(k, _)| k).collect();
        let locals: IndexSet<_> = self.locals.iter().map(|(k, _)| k).collect();
        idents
            .difference(&locals)
            .map(|k| *k)
            .collect::<IndexSet<_>>()
    }

    /// Get all the symbol identifiers in the module.
    pub fn idents(&self) -> &IndexMap<JsWord, Vec<Ident>> {
        &self.idents
    }

    /// Get all the local symbol identifiers in the module.
    pub fn locals(&self) -> &IndexMap<JsWord, Vec<LocalSymbol>> {
        &self.locals
    }
}

impl VisitAll for LocalGlobalAnalysis {
    fn visit_import_star_as_specifier(
        &mut self,
        n: &ImportStarAsSpecifier,
        _: &dyn Node,
    ) {
        let locals =
            self.locals.entry(n.local.sym.clone()).or_insert(Vec::new());
        locals.push(LocalSymbol::ImportStarAs(n.clone()));
    }

    fn visit_decl(&mut self, n: &Decl, _: &dyn Node) {
        match n {
            Decl::Fn(func) => {
                let locals = self
                    .locals
                    .entry(func.ident.sym.clone())
                    .or_insert(Vec::new());
                locals.push(LocalSymbol::FnDecl(func.clone()));
            }
            Decl::Class(class) => {
                let locals = self
                    .locals
                    .entry(class.ident.sym.clone())
                    .or_insert(Vec::new());
                locals.push(LocalSymbol::ClassDecl(class.clone()));
            }
            Decl::Var(var) => {
                let word_list = var_symbol_words(var);
                for (decl, words) in word_list.iter() {
                    for word in words {
                        let locals = self
                            .locals
                            .entry((*word).clone())
                            .or_insert(Vec::new());
                        locals.push(LocalSymbol::VarDecl((*decl).clone()));
                    }
                }
            }
            _ => { /* Ignore typescript declarations */ }
        }
    }

    fn visit_import_named_specifier(
        &mut self,
        n: &ImportNamedSpecifier,
        _: &dyn Node,
    ) {
        let locals =
            self.locals.entry(n.local.sym.clone()).or_insert(Vec::new());
        locals.push(LocalSymbol::ImportNamed(n.clone()));
    }

    fn visit_ident(&mut self, n: &Ident, _: &dyn Node) {
        let idents = self.idents.entry(n.sym.clone()).or_insert(Vec::new());
        idents.push(n.clone());
    }
}
