//! Analyze the local and global symbols for a module.
//!
//! To detect globals we need to know which symbols are local
//! so this analysis stores both locals and globals.
//!
//! Strings are interned so cloning the detected AST nodes
//! should be relatively cheap.
//!
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit, VisitAll};

/// Visit a module and detect local and global symbols.
#[derive(Default, Debug)]
pub struct LocalGlobalAnalysis {}

impl VisitAll for LocalGlobalAnalysis {
    fn visit_ident(&mut self, n: &Ident, _: &dyn Node) {

    }
}
