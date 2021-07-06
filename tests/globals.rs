use anyhow::Result;
use std::path::PathBuf;

use basalt::analysis::local_global::LocalGlobalAnalysis;

use swc_ecma_visit::VisitAllWith;

fn analyze(file: PathBuf) -> Result<LocalGlobalAnalysis> {
    let mut local_global: LocalGlobalAnalysis = Default::default();
    let (_, _, module) = basalt::swc_utils::load_file(&file)?;
    module.visit_all_children_with(&mut local_global);
    Ok(local_global)
}

#[test]
fn globals_import_named() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/import-named/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/import-named/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_import_star_as() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/import-star-as/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/import-star-as/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_function_decl() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/function-decl/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/function-decl/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_class_decl() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/class-decl/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/class-decl/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_var_decl() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/var-decl/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/var-decl/input.js"))?;
    let globals = analysis.globals();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}
