use anyhow::Result;
use std::path::PathBuf;

use basalt::static_module_record::{transform, TransformSource};

#[test]
fn import_wildcard_name() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/import-wildcard-name/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/import-wildcard-name/input.js",
    )))?;
    //println!("{}", result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn import_multiple_names() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/import-multiple-names/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/import-multiple-names/input.js",
    )))?;
    //println!("{}", result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn import_name() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/import-name/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/import-name/input.js",
    )))?;
    //println!("{}", result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
