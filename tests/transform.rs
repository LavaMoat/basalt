use anyhow::Result;
use std::path::PathBuf;

use basalt::static_module_record::{transform, TransformSource};

fn print_debug(expected: &str, code: &str) {
    println!("---");
    print!("{}", expected);
    println!("---");
    print!("{}", code);
    println!("---");
}

#[test]
fn export_default() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/output/transform/export-default.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/fixtures/transform/export-default.js",
    )))?;
    //print_debug(&expected, &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

#[test]
fn export_default_class() -> Result<()> {
    //let expected =
        //std::fs::read_to_string("tests/output/transform/export_default.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/fixtures/transform/export-default-class.js",
    )))?;
    print!("{}", result.code);
    //print_debug(&expected, &result.code);
    //assert_eq!(expected, result.code);
    Ok(())
}
