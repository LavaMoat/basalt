//! Build a package policy.

use std::path::PathBuf;

use anyhow::Result;

use super::Policy;
use crate::module::node::{parse_file, VisitedDependency, VisitedModule};

use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeResolver};

/// Generate a policy.
pub struct PolicyBuilder {
    entry: PathBuf,
    resolver: Box<dyn Resolve>,
}

impl PolicyBuilder {
    /// Create a package builder.
    pub fn new(entry: PathBuf) -> Self {
        Self {
            entry,
            resolver: Box::new(NodeResolver::default()),
        }
    }

    /// Load the entry point module and all dependencies.
    pub fn load(self) -> Result<Self> {
        let module = parse_file(&self.entry, &self.resolver)?;

        let node = match &*module {
            VisitedModule::Module(_, _, node) => Some(node),
            VisitedModule::Json(_) => None,
        };

        let visitor = |dep: VisitedDependency| {
            //println!("Visiting module dependency {:#?}", dep);
        };

        if let Some(node) = node {
            node.visit(&visitor)?;
        }

        Ok(self)
    }

    /// Generate a package policy file.
    pub fn finalize(self) -> Policy {
        Default::default()
    }
}
