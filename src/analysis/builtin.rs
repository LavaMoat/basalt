//! Analyze imports from builtin modules.
//!
use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

use indexmap::IndexSet;

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

impl Visit for BuiltinAnalysis {
    fn visit_module_item(&mut self, n: &ModuleItem, _: &dyn Node) {
        match n {
            ModuleItem::ModuleDecl(decl) => match decl {
                ModuleDecl::Import(import) => {
                    for spec in import.specifiers.iter() {
                        let _id = match spec {
                            ImportSpecifier::Named(n) => &n.local.sym,
                            ImportSpecifier::Default(n) => &n.local.sym,
                            ImportSpecifier::Namespace(n) => &n.local.sym,
                        };
                    }
                }
                _ => {}
            },
            ModuleItem::Stmt(_stmt) => {}
        }
    }
}
