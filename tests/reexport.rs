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
fn reexport() -> Result<()> {
    let expected = read_to_string("tests/transform/reexport/output.js")?;
    let (_, result) = transform("tests/transform/reexport/input.js")?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn reexport_alias() -> Result<()> {
    let expected = read_to_string("tests/transform/reexport-alias/output.js")?;
    let (_, result) = transform("tests/transform/reexport-alias/input.js")?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn reexport_default_alias() -> Result<()> {
    let expected =
        read_to_string("tests/transform/reexport-default-alias/output.js")?;
    let (_, result) =
        transform("tests/transform/reexport-default-alias/input.js")?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
