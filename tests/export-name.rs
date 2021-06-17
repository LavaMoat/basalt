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
fn export_name_4() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-4/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-name-4/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

// TODO: 5
// TODO: 6
// TODO: 7
// TODO: 8

#[test]
fn export_name_8() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-8/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-name-8/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_9() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-9/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-name-9/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

// NOTE: This test keeps the parentheses around the `class` whereas the
// NOTE: original does not. In practice this shouldn't be a problem.
#[test]
fn export_name_10() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-10/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-name-10/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

// TODO: 11

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
