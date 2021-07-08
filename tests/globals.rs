use anyhow::Result;
use std::path::PathBuf;

use basalt::analysis::block_scope::ScopeAnalysis;

use swc_ecma_visit::VisitWith;

fn analyze(file: PathBuf) -> Result<ScopeAnalysis> {
    let mut analyzer = ScopeAnalysis::new();
    let (_, _, module) = basalt::swc_utils::load_file(&file)?;
    module.visit_children_with(&mut analyzer);
    Ok(analyzer)
}

#[test]
fn globals_import_named() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/basic/import-named/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/basic/import-named/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_import_star_as() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/basic/import-star-as/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/basic/import-star-as/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_function_decl() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/basic/function-decl/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/basic/function-decl/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_class_decl() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/basic/class-decl/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/basic/class-decl/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_var_decl() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/basic/var-decl/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/basic/var-decl/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_var_destructure_decl() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/basic/var-destructure-decl/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/basic/var-destructure-decl/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}
