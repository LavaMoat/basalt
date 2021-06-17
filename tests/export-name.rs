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

// FIXME: `nest` is not rendered in the source test spec but we render it?
// FIXME: `rest` is not being rendered but exists in the source test spec?
#[test]
fn export_name_2() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-2/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-name-2/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_3() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-3/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-name-3/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_12() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-12/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-name-12/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_13() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-13/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-name-13/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
