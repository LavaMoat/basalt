//! Build a package policy.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{bail, Result};

use swc_common::FileName;
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeResolver};

use super::Policy;
use crate::analysis::dependencies::is_dependent_module;
use crate::module::node::{parse_file, VisitedDependency, VisitedModule};

/// Generate a policy.
pub struct PolicyBuilder {
    entry: PathBuf,
    resolver: Box<dyn Resolve>,
    /// Package buckets used the module specifier and the base path
    /// for the package as the key and map to all the modules inside
    /// the base path.
    package_buckets: HashMap<(String, PathBuf), HashSet<PathBuf>>,
}

impl PolicyBuilder {
    /// Create a package builder.
    pub fn new(entry: PathBuf) -> Self {
        Self {
            entry,
            resolver: Box::new(NodeResolver::default()),
            package_buckets: Default::default(),
        }
    }

    /// Load the entry point module and all dependencies.
    pub fn load(mut self) -> Result<Self> {
        let module = parse_file(&self.entry, &self.resolver)?;

        let node = match &*module {
            VisitedModule::Module(_, _, node) => Some(node),
            VisitedModule::Json(_) => None,
        };

        let mut visitor = |dep: VisitedDependency| {
            if is_dependent_module(&dep.spec) {
                match dep.file_name {
                    FileName::Real(path) => {
                        if let Some(module_base) =
                            module_base_directory(&dep.spec, &path)
                        {
                            let set = self.package_buckets
                                .entry((dep.spec.clone(), module_base))
                                .or_insert(Default::default());
                            set.insert(path);
                        } else {
                            bail!("Failed to resolve module base for specifier {}", &dep.spec);
                        }
                    }
                    _ => {}
                }
            }

            Ok(())
        };

        if let Some(node) = node {
            node.visit(&mut visitor)?;
        }

        println!("Package buckets {:#?}", self.package_buckets);

        Ok(self)
    }

    /// Generate a package policy file.
    pub fn finalize(self) -> Policy {
        Default::default()
    }
}

// Attempt to find the base directory for a module import specifier.
//
// Walks the parents of the path and matches against the specifier split on a
// slash to account for scoped packages in the specifier.
//
// WARN: Nested imports that reference a file in the package may not work as
// WARN: expected due to the use of a file name in the specifier.
//
fn module_base_directory(specifier: &str, path: &PathBuf) -> Option<PathBuf> {
    let get_requirements = || -> Vec<&str> {
        let mut requirements: Vec<&str> = specifier.split("/").collect();
        requirements.insert(0, "node_modules");
        requirements = requirements.into_iter().rev().collect();
        requirements
    };

    let mut requirements = get_requirements();
    let mut search = path.to_path_buf();

    while let Some(p) = search.parent() {
        if let Some(name) = p.file_name() {
            if let Some(needle) = requirements.get(0) {
                // This part matches the specifier
                if *needle == name.to_string_lossy().as_ref() {
                    requirements.swap_remove(0);
                    // If the requirements are empty we matched all parts
                    if requirements.is_empty() {
                        let mut base = p.to_path_buf();
                        // Append the requirements back to the current parent path
                        // skipping the `node_modules` we just matched on.
                        let mut replay: Vec<&str> = get_requirements()
                            .into_iter()
                            .rev()
                            .skip(1)
                            .collect();
                        for part in replay.drain(..) {
                            base = base.join(part);
                        }
                        return Some(base);
                    }
                }
            } else {
                // Matches must be consecutive so we reset if a match fails
                requirements = get_requirements();
            }
        }
        search = p.to_path_buf();
    }
    None
}
