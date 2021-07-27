use std::path::PathBuf;

use anyhow::Result;
use indexmap::IndexMap;
use swc_atoms::JsWord;

use basalt::{
    access::Access,
    analysis::builtin::BuiltinAnalysis,
    swc_utils::load_file,
};

fn analyze(file: &str) -> Result<IndexMap<JsWord, Access>> {
    let (_, _, module) = load_file(PathBuf::from(file))?;
    let analyzer = BuiltinAnalysis::new(Default::default());
    Ok(analyzer.analyze(&module))
}

#[test]
fn policy_builtin_write_access() -> Result<()> {
    let result =
        analyze("tests/policy/builtin/access/write/input.js")?;
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.write);
    Ok(())
}
