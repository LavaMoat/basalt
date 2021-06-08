use std::sync::Arc;
use std::path::Path;

use swc_ecma_ast::Module;

use anyhow::{bail, Result};

use spack::resolvers::NodeResolver;
use swc_bundler::Resolve;
use swc_common::{comments::SingleThreadedComments, FileName, SourceMap};

use crate::types::ModuleNode;

#[derive(Debug, Clone)]
pub struct VisitState {
    open: Vec<BranchState>,
    parents: Vec<FileName>,
    file_name: FileName,
}

impl VisitState {
    pub fn new(file_name: FileName) -> Self {
        Self {file_name, open: Vec::new(), parents: Vec::new()}
    }
}

#[derive(Debug, Default, Clone)]
struct BranchState {
    last: bool,
}

/// Parse a file, analyze dependencies and resolve dependency file paths.
pub fn parse_file<P: AsRef<Path>>(
    file: P,
    resolver: &Box<dyn Resolve>,
) -> Result<(FileName, Arc<SourceMap>, ModuleNode)> {
    let (file_name, source_map, module) =
        crate::swc_utils::load_file(file)?;
    let comments: SingleThreadedComments = Default::default();
    let mut node = ModuleNode::from(module);
    node.analyze(&source_map, &comments);
    node.resolve(resolver, &file_name)?;
    Ok((file_name, source_map, node))
}

pub struct ModuleGraph {
    root: ModuleNode,
    target: Option<ModuleNode>,
    root_emitted: bool,
    state: VisitState,
}

impl ModuleGraph {
    pub fn new<P: AsRef<Path>>(file: P) -> Result<Self> {
        let resolver: Box<dyn Resolve> = Box::new(NodeResolver::new());
        let (file_name, _, root) = parse_file(file.as_ref(), &resolver)?;
        Ok(Self { root, root_emitted: false, state: VisitState::new(file_name), target: None })
    }

}

impl Iterator for ModuleGraph {
    type Item = Result<VisitState>;
    fn next(&mut self) -> Option<Self::Item> {
        println!("Iterator next was called...");
        if !self.root_emitted {
            self.root_emitted = true;
            return Some(Ok(self.state.clone()));
        } else {
            let target = self.target.as_ref().unwrap_or(&self.root);
            if !target.is_empty() {

            }
        }
        None
    }
}
