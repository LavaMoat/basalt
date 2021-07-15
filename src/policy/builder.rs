//! Build a package policy.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::Policy;
use crate::module::cache::load_module;

/// Generate a policy.
pub struct PolicyBuilder {
    entry: PathBuf,
}

impl PolicyBuilder {
    /// Create a package builder.
    pub fn new(entry: PathBuf) -> Self {
        Self { entry }
    }

    /// Load the entry point module and all dependencies.
    pub fn load(self) -> Result<Self> {
        cache_modules(&self.entry)?;
        Ok(self)
    }

    /// Generate a package policy file.
    pub fn finalize() -> Policy {
        Default::default()
    }
}

fn cache_modules<P: AsRef<Path>>(file: P) -> Result<()> {
    let (module, _, _) = load_module(file.as_ref())?;
    Ok(())
}
