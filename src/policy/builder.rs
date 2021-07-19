//! Build a package policy.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{bail, Result};

use swc_common::{chain, FileName};
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeResolver};
use swc_ecma_visit::VisitWith;

use super::{PackagePolicy, Policy, PolicyAccess};
use crate::{
    analysis::{
        dependencies::is_dependent_module, globals_scope::GlobalAnalysis,
    },
    module::base::module_base_directory,
    module::node::{
        cached_modules, parse_file, VisitedDependency, VisitedModule,
    },
};

/// Generate a policy.
pub struct PolicyBuilder {
    entry: PathBuf,
    resolver: Box<dyn Resolve>,
    /// Package buckets used the module specifier and the base path
    /// for the package as the key and map to all the modules inside
    /// the base path.
    package_buckets: HashMap<(String, PathBuf), HashSet<PathBuf>>,
    /// Cumulative analysis for a package by merging the analysis for
    /// each module in the package.
    package_analysis: HashMap<(String, PathBuf), PackagePolicy>,
}

impl PolicyBuilder {
    /// Create a package builder.
    pub fn new(entry: PathBuf) -> Self {
        Self {
            entry,
            resolver: Box::new(NodeResolver::default()),
            package_buckets: Default::default(),
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
        };

        let mut visitor = |dep: VisitedDependency| {
            //println!("Visiting module {:?}", dep.spec);
            if is_dependent_module(&dep.spec) {
                match dep.file_name {
                    FileName::Real(path) => {
                        if let Some(module_base) =
                            module_base_directory(&dep.spec, &path)
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

    /// Analyze and aggregate the modules for all dependent packages.
    pub fn analyze(mut self) -> Result<Self> {
        let cache = cached_modules();
        for ((spec, module_base), modules) in self.package_buckets.drain() {
            // Aggregated analysis data
            let mut analysis: PackagePolicy = Default::default();

            for module_key in modules {
                if let Some(cached_module) = cache.get(&module_key) {
                    let visited_module = cached_module.value();
                    match &**visited_module {
                        VisitedModule::Module(_, _, node) => {
                            let mut globals_scope =
                                GlobalAnalysis::new(Default::default());
                            //println!("Analyze module: {}", module_key.display());

                            // TODO: chain the visitors!
                            node.module.visit_children_with(&mut globals_scope);

                            let module_globals = globals_scope.globals();
                            for atom in module_globals {
                                analysis.globals.insert(
                                    atom.as_ref().to_string(),
                                    true.into(),
                                );
                            }

                            if let Some(ref deps) = node.dependencies {
                                let mut packages: BTreeMap<
                                    String,
                                    PolicyAccess,
                                > = deps
                                    .iter()
                                    .filter_map(|dep| {
                                        if is_dependent_module(
                                            dep.specifier.as_ref(),
                                        ) {
                                            Some((
                                                dep.specifier
                                                    .as_ref()
                                                    .to_string(),
                                                true.into(),
                                            ))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                analysis.packages.append(&mut packages);
                            }
                        }
                        _ => {}
                    }
                } else {
                    bail!(
                        "Failed to locate cached module for {}",
                        module_key.display()
                    );
                }
            }

            self.package_analysis.insert((spec, module_base), analysis);
        }

        Ok(self)
    }

    /// Generate a package policy file.
    pub fn finalize(mut self) -> Policy {
        let mut policy: Policy = Default::default();
        for ((spec, _), analysis) in self.package_analysis.drain() {
            policy.insert(spec, analysis);
        }
        policy
    }
}

/*
// Attempt to find the base directory for a module import specifier.
fn module_base_directory(specifier: &str, path: &PathBuf) -> Option<PathBuf> {
    let mut sys_path = path.to_string_lossy().to_string();
    if cfg!(target_os = "windows") {
        sys_path = sys_path.replace("\\", "/");
    }
    let mut sub_path = vec!["node_modules"];
    let mut spec_parts : Vec<&str> = specifier.split("/").collect();
    if let Some(first) = spec_parts.get(0) {
        sub_path.push(first);
        if first.starts_with("@") {
            if let Some(second) = spec_parts.get(1) {
                sub_path.push(second);
            }
        }
    }
    let sub_path = sub_path.join("/");
    for i in sys_path.rmatch_indices(&sub_path).take(1) {
        let before = &sys_path[0..i.0];
        return Some(PathBuf::from(format!("{}{}", before, i.1)));
    }

    //println!("Sub path is {:#?}", sub_path);

    let needle = format!("/node_modules/{}", specifier);
    if sys_path.ends_with(&needle) {
        let root = sys_path.trim_end_matches(&needle);
        let mut root_path = PathBuf::from(root);
        root_path.push("node_modules");
        let parts: Vec<&str> = specifier.split("/").collect();
        if let Some(first) = parts.get(0) {
            root_path.push(first);
            if first.starts_with("@") {
                if let Some(second) = parts.get(1) {
                    root_path.push(second);
                }
            }
            return Some(root_path);
        }
    }

    // This handles the common path of standard package import.
    let get_requirements = || -> Vec<&str> {
        let mut requirements: Vec<&str> = specifier.split("/").collect();
        requirements.insert(0, "node_modules");
        requirements = requirements.into_iter().rev().collect();
        requirements
    };

    let mut matches: Vec<bool> = Vec::new();

    let mut requirements = get_requirements();
    let mut search = path.to_path_buf();

    while let Some(p) = search.parent() {
        if let Some(name) = p.file_name() {
            if let Some(needle) = requirements.get(0) {
                // This part matches the specifier
                if *needle == name.to_string_lossy().as_ref() {
                    println!("Got match on {:#?}", name);
                    //matches.push(true);
                    //println!("Before remove {:#?}", requirements.len());
                    //requirements.remove(0);
                    //println!("After remove {:#?}", requirements.len());
                    // If the requirements are empty we matched all parts
                    if requirements.len() == matches.len() {
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

                        println!("Returning base {:#?}", base);

                        return Some(base);
                    }
                }
            } else {
                // Matches must be consecutive so we reset if a match fails
                //requirements = get_requirements();
            }
        }
        search = p.to_path_buf();
    }
    None
}
*/
