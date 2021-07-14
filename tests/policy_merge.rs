use anyhow::Result;

use basalt::policy::{PackagePolicy, Policy};

#[test]
fn policy_merge() -> Result<()> {
    let mut policy1: Policy = Default::default();
    let mut pkg: PackagePolicy = Default::default();
    pkg.globals.insert("process.env", true.into());
    pkg.packages.insert("bar", true.into());
    policy1.insert("foo".to_string(), pkg);

    let policy_json = serde_json::to_string_pretty(&policy1)?;
    println!("{}", policy_json);

    Ok(())
}
