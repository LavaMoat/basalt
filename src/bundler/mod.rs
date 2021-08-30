//! Generate bundles.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use swc_common::SourceMap;
use swc_ecma_ast::Program;

mod builder;
mod serializer;

/// Options for bundling.
#[derive(Debug)]
pub struct BundleOptions {
    pub(crate) module: Vec<PathBuf>,
    pub(crate) policy: Vec<PathBuf>,
}

/// Generate a bundle from the given options.
pub fn bundle(options: BundleOptions) -> Result<(Program, Arc<SourceMap>)> {
    let builder = builder::BundleBuilder::new();
    Ok(builder
        .load_policy_files(&options.policy)?
        .fold()?
        .finalize())
}
