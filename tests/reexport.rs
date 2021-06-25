use anyhow::Result;
use std::path::PathBuf;

use basalt::static_module_record::{transform, TransformSource};

#[test]
fn reexport() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/reexport/output.js")?;
    let (_, result) = transform(TransformSource::File(PathBuf::from(
        "tests/transform/reexport/input.js",
    )))?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn reexport_alias() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/transform/reexport-alias/output.js")?;
    let (_, result) = transform(TransformSource::File(PathBuf::from(
        "tests/transform/reexport-alias/input.js",
    )))?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn reexport_default_alias() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/transform/reexport-default-alias/output.js",
    )?;
    let (_, result) = transform(TransformSource::File(PathBuf::from(
        "tests/transform/reexport-default-alias/input.js",
    )))?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
