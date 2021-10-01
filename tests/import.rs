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
fn import_wildcard_name() -> Result<()> {
    let expected =
        read_to_string("tests/transform/import-wildcard-name/output.js")?;
    let (_, result) =
        transform("tests/transform/import-wildcard-name/input.js")?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn import_multiple_names() -> Result<()> {
    let expected =
        read_to_string("tests/transform/import-multiple-names/output.js")?;
    let (_, result) =
        transform("tests/transform/import-multiple-names/input.js")?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn import_default() -> Result<()> {
    let expected = read_to_string("tests/transform/import-default/output.js")?;
    let (_, result) = transform("tests/transform/import-default/input.js")?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn import_default_alias() -> Result<()> {
    let expected =
        read_to_string("tests/transform/import-default-alias/output.js")?;
    let (_, result) =
        transform("tests/transform/import-default-alias/input.js")?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn import_side_effect() -> Result<()> {
    let expected =
        read_to_string("tests/transform/import-side-effect/output.js")?;
    let (_, result) = transform("tests/transform/import-side-effect/input.js")?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn import_duplicate() -> Result<()> {
    let expected =
        read_to_string("tests/transform/import-duplicate/output.js")?;
    let (_, result) = transform("tests/transform/import-duplicate/input.js")?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
