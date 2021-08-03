use anyhow::Result;
use std::path::PathBuf;

use basalt::analysis::globals_scope::GlobalAnalysis;
use basalt::swc_utils::load_file;

use swc_ecma_visit::VisitWith;

fn analyze(dir: &str) -> Result<(String, String)> {
    let base = PathBuf::from(dir);
    let input = base.join("input.js");
    let expected = std::fs::read_to_string(&base.join("output.json"))?;
    let mut analyzer = GlobalAnalysis::new(Default::default());
    let (_, _, module) = load_file(&input)?;
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
        println!("Run scope spec {:#?}", dir);
        let (expected, result) = analyze(dir)?;
        //println!("{}", result);
        assert_eq!(expected, result);
    }
    Ok(())
}

const FILTERS: &[&str] = &[
    "tests/globals/filter/intrinsics",
    "tests/globals/filter/keywords",
    "tests/globals/filter/module-exports",
    "tests/globals/filter/require",
    "tests/globals/filter/global-functions",
];

#[test]
fn globals_filters() -> Result<()> {
    for dir in FILTERS {
        println!("Run filter spec {:#?}", dir);
        let (expected, result) = analyze(dir)?;
        //println!("{}", result);
        assert_eq!(expected, result);
    }
    Ok(())
}

const EXPRESSIONS: &[&str] = &[
    //"tests/globals/expr/update",
    //"tests/globals/expr/new",
    //"tests/globals/expr/arrow-func",
    //"tests/globals/expr/async-arrow-func",
    //"tests/globals/expr/paren",
    //"tests/globals/expr/yield",
    //"tests/globals/expr/ternary",
    //"tests/globals/expr/assign",
    //"tests/globals/expr/unary",
    //"tests/globals/expr/class",
    //"tests/globals/expr/class-parameters",
    //"tests/globals/expr/array-lit",
    //"tests/globals/expr/object-lit",
    //"tests/globals/expr/function",
    //"tests/globals/expr/function-default-arguments",
    //"tests/globals/expr/private-name",
    //"tests/globals/expr/private-prop",
    //"tests/globals/expr/template",
    //"tests/globals/expr/tagged-template",
    //"tests/globals/expr/sequence",
    //"tests/globals/expr/binary",
    //"tests/globals/expr/optional-chain",
    "tests/globals/expr/member",
];

#[test]
fn globals_expressions() -> Result<()> {
    for dir in EXPRESSIONS {
        println!("Run expression spec {:#?}", dir);
        let (expected, result) = analyze(dir)?;
        //println!("{}", result);
        assert_eq!(expected, result);
    }
    Ok(())
}

const SHADOWS: &[&str] = &[
    "tests/globals/shadow/function-decl",
    "tests/globals/shadow/function-expr",
    "tests/globals/shadow/class-method",
    "tests/globals/shadow/arrow-function",
    "tests/globals/shadow/class-member",
    "tests/globals/shadow/module",
    "tests/globals/shadow/block",
];

#[test]
fn globals_shadows() -> Result<()> {
    for dir in SHADOWS {
        println!("Run shadow spec {:#?}", dir);
        let (expected, result) = analyze(dir)?;
        //println!("{}", result);
        assert_eq!(expected, result);
    }
    Ok(())
}

#[test]
fn globals_normalize_global_this() -> Result<()> {
    let (expected, result) = analyze("tests/globals/normalize/global-this")?;
    //println!("{}", result);
    assert_eq!(expected.trim_end(), result);
    Ok(())
}
