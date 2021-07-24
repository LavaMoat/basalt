//! Code access permissions.

/// Represents the access control to a code symbol.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Access {
    /// Read access to the symbol.
    pub read: bool,
    /// Wrote access to the symbol.
    pub write: bool,
    /// Execute (function call) access to the symbol.
    pub execute: bool,
}
