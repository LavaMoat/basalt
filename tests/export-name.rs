use anyhow::Result;
use std::path::PathBuf;

use basalt::static_module_record::{transform, TransformSource};

#[test]
fn export_name_1() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-1/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-name-1/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
