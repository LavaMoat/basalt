//! Generate bundles.

use anyhow::Result;
use std::path::PathBuf;

mod builder;

/// Options for bundling.
#[derive(Debug)]
pub struct BundleOptions {
    pub(crate) module: Vec<PathBuf>,
    pub(crate) policy: Vec<PathBuf>,
}

/// Generate a bundle from the given options.
pub fn bundle(options: BundleOptions) -> Result<()> {
    let builder = builder::BundleBuilder::new();
    let program = builder.load_policy_files(&options.policy)?.inject_iife().finalize();

    println!("{:#?}", program);

    Ok(())
}
