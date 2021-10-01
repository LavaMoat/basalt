use anyhow::Result;
use std::sync::Arc;

use swc::TransformOutput;
use swc_common::SourceMap;

use basalt::static_module_record::{self, StaticModuleRecordMeta};

mod common;
use common::read_to_string;

fn transform(src: &str) -> Result<(StaticModuleRecordMeta, TransformOutput)> {
    let source_map: Arc<SourceMap> = Arc::new(Default::default());
    static_module_record::transform(src.into(), source_map)
}

#[test]
fn export_default() -> Result<()> {
    let expected = read_to_string("tests/transform/export-default/output.js")?;
    let (_, result) = transform("tests/transform/export-default/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_class() -> Result<()> {
    let expected =
        read_to_string("tests/transform/export-default-class/output.js")?;
    let (_, result) =
        transform("tests/transform/export-default-class/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_function() -> Result<()> {
    let expected =
        read_to_string("tests/transform/export-default-function/output.js")?;
    let (_, result) =
        transform("tests/transform/export-default-function/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_arrow_function() -> Result<()> {
    let expected = read_to_string(
        "tests/transform/export-default-arrow-function/output.js",
    )?;
    let (_, result) =
        transform("tests/transform/export-default-arrow-function/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_number() -> Result<()> {
    let expected =
        read_to_string("tests/transform/export-default-number/output.js")?;
    let (_, result) =
        transform("tests/transform/export-default-number/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_arguments() -> Result<()> {
    let expected =
        read_to_string("tests/transform/export-default-arguments/output.js")?;
    let (_, result) =
        transform("tests/transform/export-default-arguments/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_this() -> Result<()> {
    let expected =
        read_to_string("tests/transform/export-default-this/output.js")?;
    let (_, result) =
        transform("tests/transform/export-default-this/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
