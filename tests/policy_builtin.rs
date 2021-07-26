use std::path::PathBuf;

use anyhow::Result;

use basalt::policy::builder::PolicyBuilder;

fn load_policy_test(dir: &str) -> Result<(String, String)> {
    let expected = std::fs::read_to_string(
        PathBuf::from(dir).join("output.json"),
    )?;
    let file = PathBuf::from(dir).join("input.js");
    let builder = PolicyBuilder::new(file);
    let policy = builder.load()?.analyze()?.finalize();
    let result = serde_json::to_string_pretty(&policy)?;
    Ok((expected.trim_end().to_string(), result))
}

#[test]
fn policy_builtin_esm() -> Result<()> {
    let (expected, result) = load_policy_test("tests/policy/builtin/esm")?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn policy_builtin_cjs() -> Result<()> {
    let (expected, result) = load_policy_test("tests/policy/builtin/cjs")?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}
