//! Build a package policy.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{bail, Result};

use swc_common::FileName;
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeResolver};
use swc_ecma_visit::VisitWith;

use rayon::prelude::*;

use super::{PackagePolicy, Policy, PolicyAccess};
use crate::{
    analysis::{
        builtin::BuiltinAnalysis, dependencies::is_dependent_module,
        globals_scope::GlobalAnalysis,
    },
    helpers::normalize_specifier,
    module::base::module_base_directory,
    module::node::{
        cached_modules, parse_file, VisitedDependency, VisitedModule,
    },
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
    /// Package buckets after grouping.
    package_groups: HashMap<String, HashSet<PathBuf>>,
    /// Cumulative analysis for a package by merging the analysis for
    /// each module in the package.
    //package_analysis: HashMap<(String, PathBuf), PackagePolicy>,
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
            //println!("Visiting module {:?}", dep.spec);
            if is_dependent_module(&dep.spec) {
                match dep.file_name {
                    FileName::Real(path) => {
                        if let Some(module_base) = module_base_directory(&path)
                        {
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

        // Put the cached module paths in each package bucket.
        for item in cached_modules().iter() {
            let key = item.key();
            for ((_, module_base), set) in self.package_buckets.iter_mut() {
                if key.starts_with(module_base) {
                    set.insert(key.to_path_buf());
                }
            }
        }

        Ok(self)
    }

    /// Flatten package nested paths so that the modules are grouped
    /// with the parent package.
    pub fn flatten(mut self) -> Result<Self> {
        let mut tmp: HashMap<(String, PathBuf), HashSet<PathBuf>> =
            HashMap::new();
        for ((spec, module_base), modules) in self.package_buckets.drain() {
            let key = normalize_specifier(&spec);
            let entry =
                tmp.entry((key, module_base)).or_insert(Default::default());
            for p in modules {
                entry.insert(p);
            }
        }

        self.package_buckets = tmp;

        Ok(self)
    }

    /// Merge packages with the same specifier.
    ///
    /// THe npm package manager allows multiple versions of the same package
    /// so we merge them into a single bucket with all of the modules so
    /// the end result is cumulative analysis across multiple versions of the same package.
    pub fn group(mut self) -> Result<Self> {
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
            .map(|(spec, modules)| (spec, analyze_modules(modules)))
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
    modules: HashSet<PathBuf>,
) -> Result<PackagePolicy> {
    let cache = cached_modules();

    // Aggregated analysis data
    let mut analysis: PackagePolicy = Default::default();

    let data: Vec<(
        BTreeMap<String, PolicyAccess>,
        BTreeMap<String, PolicyAccess>,
        BTreeMap<String, PolicyAccess>,
    )> = modules
        .into_par_iter()
        .map(|module_key| {
            let cached_module = cache.get(&module_key).unwrap();
            let visited_module = cached_module.value();
            if let VisitedModule::Module(_, _, node) = &**visited_module {
                // Compute and aggregate globals
                let mut globals_scope =
                    GlobalAnalysis::new(Default::default());
                node.module.visit_children_with(&mut globals_scope);
                let globals = globals_scope
                    .compute()
                    .into_iter()
                    .map(|atom| (atom.as_ref().to_string(), true.into()))
                    .collect::<BTreeMap<String, PolicyAccess>>();

                // Compute and aggregate builtins
                let builtins: BuiltinAnalysis = Default::default();
                let builtin = builtins
                    .analyze(&*node.module)
                    .into_iter()
                    .map(|(atom, _access)| {
                        (atom.as_ref().to_string(), true.into())
                    })
                    .collect::<BTreeMap<String, PolicyAccess>>();

                let packages = if let Some(deps) = &node.dependencies {
                    deps.iter()
                        .filter_map(|dep| {
                            if is_dependent_module(dep.specifier.as_ref()) {
                                Some((
                                    normalize_specifier(
                                        dep.specifier.as_ref(),
                                    ),
                                    true.into(),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect::<BTreeMap<String, PolicyAccess>>()
                } else {
                    BTreeMap::new()
                };

                return (globals, builtin, packages);
            }
            (BTreeMap::new(), BTreeMap::new(), BTreeMap::new())
        })
        .collect();

    for (mut globals, mut builtin, mut packages) in data {
        analysis.globals.append(&mut globals);
        analysis.builtin.append(&mut builtin);
        analysis.packages.append(&mut packages);
    }

    Ok(analysis)
}

