use anyhow::Result;
use std::path::PathBuf;

use basalt::static_module_record::{transform, TransformSource};

#[test]
fn live_export_assignment() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/transform/live-export-assignment/output.js",
    )?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/live-export-assignment/input.js",
    )))?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn live_export_reexport() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/transform/live-export-reexport/output.js",
    )?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/live-export-reexport/input.js",
    )))?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn live_export_reexport_alias() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/transform/live-export-reexport-alias/output.js",
    )?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/live-export-reexport-alias/input.js",
    )))?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
