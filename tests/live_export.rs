use anyhow::Result;
use std::path::PathBuf;

use basalt::static_module_record::{transform, TransformSource};

#[test]
fn live_export_assignment() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/transform/live-export-assignment/output.js",
    )?;
    let (_, result) = transform(TransformSource::File(PathBuf::from(
        "tests/transform/live-export-assignment/input.js",
    )))?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
