//! Generate bundles.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use swc_common::SourceMap;
use swc_ecma_ast::Program;

mod builder;
mod loader;
mod serializer;

/// Options for bundling.
#[derive(Debug)]
pub struct BundleOptions {
    pub(crate) module: PathBuf,
    pub(crate) policy: Vec<PathBuf>,
}

/// Generate a bundle from the given options.
pub fn bundle(options: BundleOptions) -> Result<(Program, Arc<SourceMap>)> {
    let builder = builder::BundleBuilder::new();
    let module = options
        .module
        .canonicalize()
        .context("Failed to determine canonical path for module entry point")?;
    Ok(builder
        .load_policy_files(&options.policy)?
        .fold(module)?
        .finalize())
}
