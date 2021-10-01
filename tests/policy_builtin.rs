use std::path::PathBuf;

use anyhow::Result;

use basalt::policy::builder::PolicyBuilder;

mod common;
use common::read_to_string;

fn load_policy_test(dir: &str) -> Result<(String, String)> {
    let expected =
        read_to_string(PathBuf::from(dir).join("output.json"))?;
    let file = PathBuf::from(dir).join("input.js");
    let builder = PolicyBuilder::new(file);
    let policy = builder.load()?.analyze()?.finalize();
    let result = serde_json::to_string_pretty(&policy)?;
    Ok((expected.trim_end().to_string(), result))
}

const MODULES: &[&str] = &[
    "tests/policy/builtin/esm",
    "tests/policy/builtin/cjs",
    "tests/policy/builtin/named-import",
    "tests/policy/builtin/named-require",
    "tests/policy/builtin/named-deep",
    "tests/policy/builtin/binary-expression",
];

#[test]
fn policy_builtin_resources() -> Result<()> {
    for dir in MODULES {
        println!("Run policy builtin spec {:#?}", dir);
        let (expected, result) = load_policy_test(dir)?;
        //println!("{}", result);
        assert_eq!(expected, result);
    }
    Ok(())
}
