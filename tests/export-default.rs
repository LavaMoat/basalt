use anyhow::Result;
use std::path::PathBuf;

use basalt::static_module_record::{transform, TransformSource};

#[test]
fn export_default() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-default/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-default/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_class() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-default-class/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-default-class/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_function() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-default-function/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-default-function/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_arrow_function() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-default-arrow-function/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-default-arrow-function/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_number() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-default-number/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-default-number/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_arguments() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-default-arguments/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-default-arguments/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_this() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-default-this/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-default-this/input.js",
    )))?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

