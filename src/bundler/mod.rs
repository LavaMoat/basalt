//! Generate bundles.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::policy::{Merge, Policy};

/// Options for bundling.
#[derive(Debug)]
pub struct BundleOptions {
    pub(crate) module: Vec<PathBuf>,
    pub(crate) policy: Vec<PathBuf>,
}

/// Generate a bundle from the given options.
pub fn bundle(options: BundleOptions) -> Result<()> {
    let policy = load_policy_files(&options.policy)?;
    log::debug!("{:#?}", policy);

    Ok(())
}

/// Load and merge all referenced policy files
/// in the order declared.
fn load_policy_files(policy: &Vec<PathBuf>) -> Result<Policy> {
    let mut root_policy: Policy = Default::default();
    for file in policy {
        let f = File::open(file).context(format!(
            "Unable to open policy file {}",
            file.display()
        ))?;
        let reader = BufReader::new(f);
        let mut policy: Policy = serde_json::from_reader(reader)
            .context(format!("Failed to parse JSON in {}", file.display()))?;
        root_policy.merge(&mut policy);
    }
    Ok(root_policy)
}
