use anyhow::Result;
use std::path::PathBuf;

use basalt::analysis::globals_scope::GlobalAnalysis;

use swc_ecma_visit::VisitWith;

fn analyze(file: PathBuf) -> Result<GlobalAnalysis> {
    let mut analyzer = GlobalAnalysis::new(Default::default());
    let (_, _, module) = basalt::swc_utils::load_file(&file)?;
    module.visit_children_with(&mut analyzer);
    Ok(analyzer)
}

fn analyze2(dir: &str) -> Result<(String, String)> {
    let base = PathBuf::from(dir);
    let expected = base.join("output.json");
    let input = base.join("input.js");

    let expected =
        std::fs::read_to_string(&expected)?;

    let mut analyzer = GlobalAnalysis::new(Default::default());
    let (_, _, module) = basalt::swc_utils::load_file(&input)?;
    module.visit_children_with(&mut analyzer);
    let globals = analyzer.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    Ok((expected.trim_end().to_owned(), result))
}

const SCOPES: &[&str] = &[
    "tests/globals/scope/block-body",
    "tests/globals/scope/function-body",
    "tests/globals/scope/with-body",
    "tests/globals/scope/switch-case",
    "tests/globals/scope/while-body",
    "tests/globals/scope/do-while-body",
    "tests/globals/scope/for-body",
    "tests/globals/scope/for-in-body",
    "tests/globals/scope/for-of-body",
    "tests/globals/scope/if-else-if-else",
    "tests/globals/scope/try-catch-finally",
];

#[test]
fn globals_scopes() -> Result<()> {
    for dir in SCOPES {
        let (expected, result) = analyze2(dir)?;
        //println!("{}", result);
        assert_eq!(expected, result);
    }
    Ok(())
}

#[test]
fn globals_expr_update() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/update/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/update/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_new() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/new/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/expr/new/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_arrow_func() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/arrow-func/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/arrow-func/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_async_arrow_func() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/expr/async-arrow-func/output.json",
    )?;
    let analysis = analyze(PathBuf::from(
        "tests/globals/expr/async-arrow-func/input.js",
    ))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_paren() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/paren/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/expr/paren/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_yield() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/yield/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/expr/yield/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_ternary() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/ternary/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/ternary/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_assign() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/assign/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/assign/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_unary() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/unary/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/expr/unary/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_class() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/class/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/expr/class/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_class_parameters() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/class-parameters/output.json")?;
    let analysis = analyze(PathBuf::from("tests/globals/expr/class-parameters/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_array_lit() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/array-lit/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/array-lit/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_object_lit() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/object-lit/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/object-lit/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_function() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/function/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/function/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_function_default_arguments() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/expr/function-default-arguments/output.json",
    )?;
    let analysis = analyze(PathBuf::from(
        "tests/globals/expr/function-default-arguments/input.js",
    ))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_private_name() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/private-name/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/private-name/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_private_prop() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/private-prop/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/private-prop/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_template() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/template/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/template/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_tagged_template() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/expr/tagged-template/output.json",
    )?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/tagged-template/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_sequence() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/sequence/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/sequence/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_binary() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/binary/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/binary/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_member() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/expr/member/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/member/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_expr_optional_chain() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/expr/optional-chain/output.json",
    )?;
    let analysis =
        analyze(PathBuf::from("tests/globals/expr/optional-chain/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_shadow_function_decl() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/shadow/function-decl/output.json",
    )?;
    let analysis =
        analyze(PathBuf::from("tests/globals/shadow/function-decl/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_shadow_function_expr() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/shadow/function-expr/output.json",
    )?;
    let analysis =
        analyze(PathBuf::from("tests/globals/shadow/function-expr/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_shadow_class_method() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/shadow/class-method/output.json",
    )?;
    let analysis =
        analyze(PathBuf::from("tests/globals/shadow/class-method/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_shadow_arrow_function() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/shadow/arrow-function/output.json",
    )?;
    let analysis = analyze(PathBuf::from(
        "tests/globals/shadow/arrow-function/input.js",
    ))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_shadow_class_member() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/shadow/class-member/output.json",
    )?;
    let analysis =
        analyze(PathBuf::from("tests/globals/shadow/class-member/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_shadow_module() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/shadow/module/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/shadow/module/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_shadow_block() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/shadow/block/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/shadow/block/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_filter_intrinsics() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/filter/intrinsics/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/filter/intrinsics/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_filter_keywords() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/filter/keywords/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/filter/keywords/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_filter_module_exports() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/filter/module-exports/output.json",
    )?;
    let analysis = analyze(PathBuf::from(
        "tests/globals/filter/module-exports/input.js",
    ))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_filter_require() -> Result<()> {
    let expected =
        std::fs::read_to_string("tests/globals/filter/require/output.json")?;
    let analysis =
        analyze(PathBuf::from("tests/globals/filter/require/input.js"))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_filter_global_functions() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/filter/global-functions/output.json",
    )?;
    let analysis = analyze(PathBuf::from(
        "tests/globals/filter/global-functions/input.js",
    ))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}

#[test]
fn globals_normalize_global_this() -> Result<()> {
    let expected = std::fs::read_to_string(
        "tests/globals/normalize/global-this/output.json",
    )?;
    let analysis = analyze(PathBuf::from(
        "tests/globals/normalize/global-this/input.js",
    ))?;
    let globals = analysis.compute();
    let result = serde_json::to_string_pretty(&globals)?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}
