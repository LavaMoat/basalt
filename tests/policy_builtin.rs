use std::path::PathBuf;

use anyhow::Result;

use basalt::policy::builder::PolicyBuilder;

#[test]
fn policy_builtin_esm() -> Result<()> {
    let file = PathBuf::from("tests/policy-builtin/esm/input.js");

    let builder = PolicyBuilder::new(file);
    let policy = builder.load()?.analyze()?.finalize();
    let policy_content = serde_json::to_string_pretty(&policy)?;
    println!("{}", policy_content);

    Ok(())
}
