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
        std::fs::read_to_string("tests/transform/export-default/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-default/input.js",
    )))?;
    //print_debug(&expected, &result.code);
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
    //print!("{}", result.code);
    //print_debug(&expected, &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}

/*
#[test]
fn export_default_class() -> Result<()> {
    //let expected =
        //std::fs::read_to_string("tests/transform/export-default-class/output.js")?;
    let result = transform(TransformSource::File(PathBuf::from(
        "tests/transform/export-default-class/input.js",
    )))?;
    print!("{}", result.code);
    //print_debug(&expected, &result.code);
    //assert_eq!(expected, result.code);
    Ok(())
}
*/
