//! Wrapper type for module AST nodes.
//!
//! Encapsulates additional useful information and provides an
//! iterator for resolving and loading dependencies.
//!

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use swc_common::{comments::SingleThreadedComments, FileName, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};

use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeResolver};

use crate::module::cache::load_module;

/// Stores the data for a visited module dependency.
pub enum VisitedModule {
    /// A Javascript module.
    Module(FileName, Arc<SourceMap>, ModuleNode),
    /// A JSON module.
    Json(FileName),
}

/// Represents a visited dependency.
#[derive(Debug)]
pub struct VisitedDependency<'a> {
    /// The dependency specifier.
    pub spec: String,
    /// The file name for the dependency.
    pub file_name: FileName,
    /// Whether this dependency is the last child.
    pub last: bool,
    /// A parsed module for the dependency.
    pub node: &'a Option<ModuleNode>,
    /// The current visitor state.
    pub state: &'a VisitState,
    /// Indicates whether a cycle has been detected.
    pub cycles: Option<&'a FileName>,
}

/// Stores the branch state for the tree printer.
#[derive(Debug)]
pub struct BranchState {
    /// Determine if this node is the last child
    /// of it's parent.
    pub last: bool,
}

/// Represents the visit state for a node iterator.
#[derive(Debug)]
pub struct VisitState {
    /// Stack of branch states.
    pub open: Vec<BranchState>,
    /// Stack of parents when visiting a module graph.
    ///
    /// Used to detect cycles.
    pub parents: Vec<FileName>,
}

fn parse_module<P: AsRef<Path>>(
    file: P,
    resolver: &Box<dyn Resolve>,
) -> Result<VisitedModule> {
    let (module, source_map, file_name) = load_module(file)?;
    let comments: SingleThreadedComments = Default::default();
    let mut node = ModuleNode::from(module);
    node.analyze(&source_map, &comments);
    node.resolve(resolver, &file_name)?;
    Ok(VisitedModule::Module(
        (&*file_name).clone(),
        source_map,
        node,
    ))
}

/// Parse a file, analyze dependencies and resolve dependency file paths.
pub fn parse_file<P: AsRef<Path>>(
    file: P,
    resolver: &Box<dyn Resolve>,
) -> Result<VisitedModule> {
    let extension = file
        .as_ref()
        .extension()
        .map(|s| s.to_string_lossy().to_string());
    if let Some(ref extension) = extension {
        let extension = &extension[..];
        match extension {
            "json" => Ok(VisitedModule::Json(FileName::Real(
                file.as_ref().to_path_buf(),
            ))),
            _ => parse_module(file, resolver),
        }
    } else {
        parse_module(file, resolver)
    }
}

/// Encapsulates a module and it's dependencies.
#[derive(Debug)]
pub struct ModuleNode {
    /// The underlying module AST node.
    pub module: Arc<Module>,
    /// The parsed dependencies of this module.
    pub dependencies: Option<Vec<DependencyDescriptor>>,
    /// The resolved paths for the dependencies.
    pub resolved: Vec<(String, FileName)>,
}

impl ModuleNode {
    /// Analyze the dependencies for this module.
    pub fn analyze(
        &mut self,
        source_map: &SourceMap,
        comments: &SingleThreadedComments,
    ) {
        let deps = analyze_dependencies(&self.module, source_map, comments);
        self.dependencies = if deps.is_empty() { None } else { Some(deps) };
    }

    /// Resolve the dependencies for this module.
    pub fn resolve(
        &mut self,
        resolver: &Box<dyn Resolve>,
        base: &FileName,
    ) -> Result<()> {
        if let Some(deps) = &self.dependencies {
            for dep in deps {
                let spec = format!("{}", dep.specifier);
                let file_name = resolver.resolve(base, &spec).context(
                    format!("Failed to resolve module for {}", &spec),
                )?;
                self.resolved.push((spec, file_name));
            }
        }
        Ok(())
    }

    /// Iterate the resolved dependencies of this module and
    /// attempt to load a module for each resolved dependency.
    pub fn iter<'a>(&'a self) -> NodeIterator<'a> {
        NodeIterator {
            node: self,
            index: 0,
            resolver: Box::new(NodeResolver::default()),
        }
    }

    /// Visit all dependencies of this node recursively.
    pub fn visit<F>(&self, callback: &F) -> Result<()>
    where
        F: Fn(VisitedDependency),
    {
        let mut state = VisitState {
            open: Vec::new(),
            parents: Vec::new(),
        };
        self.visit_all(self, &mut state, callback)
    }

    fn visit_all<F>(
        &self,
        node: &ModuleNode,
        state: &mut VisitState,
        callback: &F,
    ) -> Result<()>
    where
        F: Fn(VisitedDependency),
    {
        state.open.push(BranchState { last: false });

        for res in node.iter() {
            let (i, spec, parsed) = res?;
            let last = i == (node.resolved.len() - 1);
            state.open.last_mut().unwrap().last = last;

            let (file_name, dep) = match parsed {
                VisitedModule::Module(file_name, _, dep) => {
                    (file_name, Some(dep))
                }
                VisitedModule::Json(file_name) => (file_name, None),
            };

            let cycles = state.parents.iter().find(|p| *p == &file_name);

            let dependency = VisitedDependency {
                spec,
                file_name: file_name.clone(),
                last: i == node.resolved.len() - 1,
                node: &dep,
                state,
                cycles,
            };

            callback.call((dependency,));

            if cycles.is_some() {
                continue;
            }

            if let Some(dep) = dep {
                if !dep.resolved.is_empty() {
                    state.parents.push(file_name);
                    self.visit_all(&dep, state, callback)?;
                    state.parents.pop();
                }
            }
        }

        state.open.pop();

        Ok(())
    }
}

impl From<Arc<Module>> for ModuleNode {
    fn from(module: Arc<Module>) -> Self {
        ModuleNode {
            module,
            dependencies: None,
            resolved: Vec::new(),
        }
    }
}

/// Iterate the resolved dependencies of a module node.
pub struct NodeIterator<'a> {
    node: &'a ModuleNode,
    resolver: Box<dyn Resolve>,
    index: usize,
}

impl<'a> Iterator for NodeIterator<'a> {
    type Item = Result<(usize, String, VisitedModule)>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.node.resolved.len() {
            return None;
        }

        if let Some(resolved) = self.node.resolved.get(self.index) {
            self.index += 1;
            if let FileName::Real(file_name) = &resolved.1 {
                return match parse_file(file_name, &self.resolver) {
                    Ok(parsed) => {
                        Some(Ok((self.index - 1, resolved.0.clone(), parsed)))
                    }
                    Err(e) => Some(Err(anyhow!(e))),
                };
            }
        }

        None
    }
}
