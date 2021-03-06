use anyhow::Result;
use std::path::PathBuf;

use basalt::policy::{Merge, Policy};

use testing::read_to_string;

#[test]
fn policy_merge() -> Result<()> {
    let expected: Policy = serde_json::from_str(&read_to_string(
        PathBuf::from("tests/policy/merge/output.json"),
    )?)?;
    let mut policy1: Policy = serde_json::from_str(&read_to_string(
        PathBuf::from("tests/policy/merge/policy1.json"),
    )?)?;
    let policy2: Policy = serde_json::from_str(&read_to_string(
        PathBuf::from("tests/policy/merge/policy2.json"),
    )?)?;

    policy1.merge(&policy2);
    assert_eq!(expected, policy1);

    Ok(())
}
