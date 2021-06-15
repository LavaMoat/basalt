use anyhow::Result;
use std::path::PathBuf;

use basalt::static_module_record::{transform, TransformSource};

#[test]
fn export_default() -> Result<()> {
    let expected = std::fs::read_to_string("tests/output/export_default.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/fixtures/smr/export_default.js",
    )))?;
    //println!("---");
    //print!("{}", expected);
    //println!("---");
    //print!("{}", result.code);
    //println!("---");
    assert_eq!(expected, result.code);
    Ok(())
}
