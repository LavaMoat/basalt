//! Generate bundles.

use anyhow::Result;
use std::path::PathBuf;
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
pub fn bundle(options: BundleOptions) -> Result<Program> {
    let builder = builder::BundleBuilder::new();
    Ok(builder
        .load_policy_files(&options.policy)?
        .inject_iife()
        .inject_policy()?
        .finalize())
}
