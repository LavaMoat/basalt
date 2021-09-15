use anyhow::Result;
use std::sync::Arc;

use swc::TransformOutput;
use swc_common::SourceMap;

use basalt::static_module_record::{self, StaticModuleRecordMeta};

fn transform(src: &str) -> Result<(StaticModuleRecordMeta, TransformOutput)> {
    let source_map: Arc<SourceMap> = Arc::new(Default::default());
    static_module_record::transform(src.into(), source_map)
}

#[test]
fn export_name_1() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-1/output.js")?;
    let (_, result) = transform("tests/transform/export-name-1/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_2() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-2/output.js")?;
    let (_, result) = transform("tests/transform/export-name-2/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_3() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-3/output.js")?;
    let (_, result) = transform("tests/transform/export-name-3/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_4() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-4/output.js")?;
    let (_, result) = transform("tests/transform/export-name-4/input.js")?;
    print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_5() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-5/output.js")?;
    let (_, result) = transform("tests/transform/export-name-5/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_6() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-6/output.js")?;
    let (_, result) = transform("tests/transform/export-name-6/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_7() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-7/output.js")?;
    let (_, result) = transform("tests/transform/export-name-7/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_8() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-8/output.js")?;
    let (_, result) = transform("tests/transform/export-name-8/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_9() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-9/output.js")?;
    let (_, result) = transform("tests/transform/export-name-9/input.js")?;
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
    let (_, result) = transform("tests/transform/export-name-10/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

// TODO: 11

#[test]
fn export_name_12() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-12/output.js")?;
    let (_, result) = transform("tests/transform/export-name-12/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_name_13() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/export-name-13/output.js")?;
    let (_, result) = transform("tests/transform/export-name-13/input.js")?;
    //print!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
