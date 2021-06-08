use std::sync::Arc;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use swc_bundler::Resolve;
use spack::resolvers::NodeResolver;
use swc_common::{comments::SingleThreadedComments, FileName, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};

pub type ParsedModule = (FileName, Arc<SourceMap>, ModuleNode);

#[derive(Debug)]
pub struct VisitedDependency<'a> {
    pub spec: String,
    pub file_name: FileName,
    pub last: bool,
    pub node: &'a ModuleNode,
    pub state: &'a VisitState,
    pub cycles: Option<&'a FileName>,
}

#[derive(Debug)]
pub struct BranchState {
    pub last: bool,
}

#[derive(Debug)]
pub struct VisitState {
    pub open: Vec<BranchState>,
    pub parents: Vec<FileName>,
}

/// Parse a file, analyze dependencies and resolve dependency file paths.
pub fn parse_file<P: AsRef<Path>>(
    file: P,
    resolver: &Box<dyn Resolve>,
) -> Result<ParsedModule> {
    let (file_name, source_map, module) =
        crate::swc_utils::load_file(file)?;
    let comments: SingleThreadedComments = Default::default();
    let mut node = ModuleNode::from(module);
    node.analyze(&source_map, &comments);
    node.resolve(resolver, &file_name)?;
    Ok((file_name, source_map, node))
}


#[derive(Debug)]
pub struct ModuleNode {
    pub module: Module,
    pub dependencies: Option<Vec<DependencyDescriptor>>,
    pub resolved: Vec<(String, FileName)>,
}

impl ModuleNode {
    pub fn analyze(
        &mut self,
        source_map: &SourceMap,
        comments: &SingleThreadedComments,
    ) {
        let deps = analyze_dependencies(&self.module, source_map, comments);
        self.dependencies = if deps.is_empty() { None } else { Some(deps) };
    }

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

    pub fn iter<'a>(&'a self) -> NodeIterator<'a> {
        NodeIterator { node: self, index: 0, resolver: Box::new(NodeResolver::new()) }
    }

    /// Visit all dependencies of this node recursively.
    pub fn visit<F>(&self, callback: &F) -> Result<()>
        where F: Fn(VisitedDependency) {
        let mut state = VisitState { open: Vec::new(), parents: Vec::new() };
        self.visit_all(self, &mut state, callback)
    }

    fn visit_all<F>(&self, node: &ModuleNode, state: &mut VisitState, callback: &F) -> Result<()>
        where F: Fn(VisitedDependency) {

        state.open.push(BranchState { last: false });

        for res in node.iter() {
            let (i, spec, (file_name, _, dep)) = res?;
            let last = i == (node.resolved.len() - 1);
            let cycles = state.parents.iter().find(|p| *p == &file_name);
            state.open.last_mut().unwrap().last = last;

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

            if !dep.resolved.is_empty() {
                state.parents.push(file_name);
                self.visit_all(&dep, state, callback)?;
                state.parents.pop();
            }
        }

        state.open.pop();

        Ok(())
    }
}

impl From<Module> for ModuleNode {
    fn from(module: Module) -> Self {
        ModuleNode {
            module,
            dependencies: None,
            resolved: Vec::new(),
        }
    }
}

pub struct NodeIterator<'a> {
    node: &'a ModuleNode,
    resolver: Box<dyn Resolve>,
    index: usize,
}

impl<'a> Iterator for NodeIterator<'a> {
    type Item = Result<(usize, String, ParsedModule)>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.node.resolved.len() {
            return None;
        }

        if let Some(resolved) = self.node.resolved.get(self.index) {
            self.index += 1;
            if let FileName::Real(file_name) = &resolved.1 {
                return match parse_file(file_name, &self.resolver) {
                    Ok(parsed) => Some(Ok((self.index - 1, resolved.0.clone(), parsed))),
                    Err(e) => Some(Err(anyhow!(e))),
                }
            }
        }

        None
    }
}
