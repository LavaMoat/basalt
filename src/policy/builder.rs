//! Build a package policy.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{bail, Result};
use indexmap::IndexSet;

use swc_atoms::JsWord;
use swc_common::FileName;
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeResolver};
use swc_ecma_visit::VisitWith;

use rayon::prelude::*;

use super::{PackagePolicy, Policy, PolicyAccess};
use crate::{
    helpers::normalize_specifier,
    module::{
        base::module_base_directory,
        dependencies::is_dependent_module,
        node::{cached_modules, parse_file, VisitedDependency, VisitedModule},
    },
    policy::analysis::{flatten, globals_scope::GlobalAnalysis, join_words},
};

/// Generate a policy.
///
/// This needs to determine a base path for each module so that we
/// can group modules to the package that they belong to in order
/// to convert a list of all modules into a collection of packages.
pub struct PolicyBuilder {
    entry: PathBuf,
    resolver: Box<dyn Resolve>,
    /// Package buckets used the module specifier and the base path
    /// for the package as the key and map to all the modules inside
    /// the base path.
    package_buckets: HashMap<(String, PathBuf), HashSet<PathBuf>>,

    /// Package buckets after grouping multiple versions of the same package.
    package_groups: HashMap<String, HashSet<PathBuf>>,

    /// Cumulative analysis for a package by merging the analysis for
    /// each module in the package.
    package_analysis: Policy,
}

impl PolicyBuilder {
    /// Create a package builder.
    pub fn new(entry: PathBuf) -> Self {
        Self {
            entry,
            resolver: Box::new(NodeResolver::default()),
            package_buckets: Default::default(),
            package_groups: Default::default(),
            package_analysis: Default::default(),
        }
    }

    /// Load the entry point module and all dependencies grouping modules
    /// into dependent package buckets.
    pub fn load(mut self) -> Result<Self> {
        let module = parse_file(&self.entry, &self.resolver)?;

        let node = match &*module {
            VisitedModule::Module(_, _, node) => Some(node),
            VisitedModule::Json(_) => None,
            VisitedModule::Builtin(_) => None,
        };

        let mut visitor = |dep: VisitedDependency| {
            if is_dependent_module(&dep.spec) {
                match dep.file_name {
                    FileName::Real(path) => {
                        if let Some(module_base) = module_base_directory(&path)
                        {
                            log::debug!(
                                "Resolved {:#?} with {:#?}",
                                &dep.spec,
                                module_base.display()
                            );
                            self.package_buckets
                                .entry((dep.spec.clone(), module_base))
                                .or_insert(Default::default());
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

        // Sort the module base keys as we need to find the deepest match
        // so the sort and reverse iteration will yield the deeper path first.
        let mut base_keys: Vec<PathBuf> = self
            .package_buckets
            .iter()
            .map(|((_, module_base), _)| module_base.clone())
            .collect();
        base_keys.sort();

        // Put the cached module paths in each package bucket.
        for item in cached_modules().iter() {
            let key = item.key();
            if let Some(module_base) =
                base_keys.iter().rev().find(|p| key.starts_with(p))
            {
                if let Some((_, modules)) = self
                    .package_buckets
                    .iter_mut()
                    .find(|((_, base), _)| base == module_base)
                {
                    modules.insert(key.to_path_buf());
                }
            }
        }

        Ok(self.flatten()?.group()?)
    }

    /// Flatten package nested paths so that the modules are grouped
    /// with the parent package.
    fn flatten(mut self) -> Result<Self> {
        let mut tmp: HashMap<(String, PathBuf), HashSet<PathBuf>> =
            HashMap::new();
        for ((spec, module_base), modules) in self.package_buckets.drain() {
            let key = normalize_specifier(&spec);
            let entry = tmp
                .entry((key, module_base.clone()))
                .or_insert(Default::default());
            for p in modules {
                entry.insert(p);
            }
        }

        self.package_buckets = tmp;

        Ok(self)
    }

    /// Merge packages with the same specifier.
    ///
    /// The npm package manager allows multiple versions of the same package
    /// so we merge them into a single bucket with all of the modules so
    /// the end result is cumulative analysis across multiple versions of the same package.
    fn group(mut self) -> Result<Self> {
        for ((spec, _module_base), modules) in self.package_buckets.drain() {
            if let Some(entry) = self.package_groups.get_mut(&spec) {
                for p in modules {
                    entry.insert(p);
                }
            } else {
                self.package_groups.insert(spec, modules);
            }
        }
        Ok(self)
    }

    /// Analyze and aggregate the modules for all dependent packages.
    pub fn analyze(mut self) -> Result<Self> {
        let groups = std::mem::take(&mut self.package_groups);

        let analyzed: Vec<_> = groups
            .into_par_iter()
            .map(|(spec, modules)| {
                let result = analyze_modules(&spec, modules);
                (spec, result)
            })
            .collect();

        for (spec, policy) in analyzed {
            let analysis = policy?;
            if !analysis.is_empty() {
                self.package_analysis.insert(spec, analysis);
            }
        }

        Ok(self)
    }

    /// Generate a package policy file.
    pub fn finalize(self) -> Policy {
        self.package_analysis
    }
}

/// Walk all the modules in a package and perform a cumulative analysis.
fn analyze_modules(
    spec: &str,
    modules: HashSet<PathBuf>,
) -> Result<PackagePolicy> {
    let cache = cached_modules();

    // Aggregated analysis data
    let mut analysis: PackagePolicy = Default::default();

    let data: Vec<(
        IndexSet<Vec<JsWord>>,
        IndexSet<Vec<JsWord>>,
        IndexSet<String>,
    )> = modules
        .into_par_iter()
        .map(|module_key| {
            let cached_module = cache.get(&module_key).unwrap();
            let visited_module = cached_module.value();
            if let VisitedModule::Module(_, _, node) = &**visited_module {
                // Compute globals
                let mut globals_scope = GlobalAnalysis::new(Default::default());
                node.module.visit_children_with(&mut globals_scope);
                let globals = globals_scope.compute_globals();

                // Compute builtins
                let builtin = globals_scope.compute_builtins();

                // Compute dependent packages
                let packages = if let Some(deps) = &node.dependencies {
                    deps.iter()
                        .filter_map(|dep| {
                            let normalized =
                                normalize_specifier(dep.specifier.as_ref());
                            // Some packages such as @babel/runtime can end up with
                            // themselves in the dependency list so we explicitly disallow this
                            if spec != &normalized
                                && is_dependent_module(dep.specifier.as_ref())
                            {
                                Some(normalized)
                            } else {
                                None
                            }
                        })
                        .collect::<IndexSet<String>>()
                } else {
                    IndexSet::new()
                };

                return (globals, builtin, packages);
            }
            (IndexSet::new(), IndexSet::new(), IndexSet::new())
        })
        .collect();

    // Group the computations for each package
    let mut pkg_globals = IndexSet::new();
    let mut pkg_builtin = IndexSet::new();
    let mut pkg_packages = IndexSet::new();

    for (globals, builtin, packages) in data {
        pkg_globals = pkg_globals.union(&globals).cloned().collect();
        pkg_builtin = pkg_builtin.union(&builtin).cloned().collect();
        pkg_packages = pkg_packages.union(&packages).cloned().collect();
    }

    // Flatten globals and builtins
    pkg_globals = flatten(pkg_globals);
    pkg_builtin = flatten(pkg_builtin);

    // Build the maps for the policy file
    let mut globals_map: BTreeMap<String, PolicyAccess> = pkg_globals
        .into_iter()
        .map(|words| (join_words(&words).as_ref().to_string(), true.into()))
        .collect();

    let mut builtin_map: BTreeMap<String, PolicyAccess> = pkg_builtin
        .into_iter()
        .map(|words| (join_words(&words).as_ref().to_string(), true.into()))
        .collect();

    let mut packages_map: BTreeMap<String, PolicyAccess> = pkg_packages
        .into_iter()
        .map(|key| (key, true.into()))
        .collect();

    analysis.globals.append(&mut globals_map);
    analysis.builtin.append(&mut builtin_map);
    analysis.packages.append(&mut packages_map);

    Ok(analysis)
}
