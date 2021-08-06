//! Symbol access flags.

/// Represents the access control to a code symbol.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Access {
    /// Read access to the symbol.
    pub read: bool,
    /// Write access to the symbol.
    pub write: bool,
    /// Execute (function call) access to the symbol.
    pub execute: bool,
}

impl Access {
    /// Merge positive flags from other into this access.
    pub fn merge(&mut self, other: &Self) {
        if other.read {
            self.read = true
        }
        if other.write {
            self.write = true
        }
        if other.execute {
            self.execute = true
        }
    }
}

/// Helper for analysis tasks to determine what type of
/// access to assign when walking AST nodes is complete.
pub enum AccessKind {
    /// Assign read access.
    Read,
    /// Assign write access.
    Write,
    /// Assign execute access.
    Execute,
}
