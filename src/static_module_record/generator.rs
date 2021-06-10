//! Generator the functor program from a static module record meta data.
use anyhow::Result;

use swc_ecma_ast::*;

use super::StaticModuleRecord;

/// Generate a static module record functor program.
pub struct Generator<'a> {
    meta: &'a StaticModuleRecord,
}

impl<'a> Generator<'a> {
    /// Create a new generator.
    pub fn new(meta: &'a StaticModuleRecord) -> Self {
        Generator { meta }
    }

    /// Create the program script AST node.
    pub fn create(&self) -> Result<()> {
        Ok(())
    }
}
