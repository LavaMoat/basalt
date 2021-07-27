use anyhow::Result;
use indexmap::IndexMap;
use swc_atoms::JsWord;

use basalt::{
    access::Access, analysis::builtin::BuiltinAnalysis, swc_utils::load_code,
};

fn analyze(code: &str) -> Result<IndexMap<JsWord, Access>> {
    let (_, _, module) = load_code(code, None)?;
    let analyzer = BuiltinAnalysis::new(Default::default());
    Ok(analyzer.analyze(&module))
}

#[test]
fn policy_builtin_access_write() -> Result<()> {
    let code = r#"process.env.FOO = 1;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.write);
    Ok(())
}

#[test]
fn policy_builtin_access_write_update() -> Result<()> {
    let code = r#"process.env.FOO++;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.write);
    Ok(())
}

#[test]
fn policy_builtin_access_execute_default_import() -> Result<()> {
    let code = r#"import fs from 'fs'; fs.readSync('foo.txt');"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("fs.readSync")).unwrap();
    assert_eq!(true, access.execute);
    Ok(())
}

#[test]
fn policy_builtin_access_execute_default_require() -> Result<()> {
    let code = r#"const fs = require('fs'); fs.readSync('foo.txt');"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("fs.readSync")).unwrap();
    assert_eq!(true, access.execute);
    Ok(())
}

#[test]
fn policy_builtin_access_execute_named_import() -> Result<()> {
    let code = r#"import {readSync} from 'fs'; readSync('foo.txt');"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("fs.readSync")).unwrap();
    assert_eq!(true, access.execute);
    Ok(())
}

#[test]
fn policy_builtin_access_execute_named_require() -> Result<()> {
    let code = r#"const {readSync} = require('fs'); readSync('foo.txt');"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("fs.readSync")).unwrap();
    assert_eq!(true, access.execute);
    Ok(())
}

#[test]
fn policy_builtin_access_execute_default_named_import() -> Result<()> {
    let code = r#"import fs, {readSync} from 'fs';
fs.writeSync('foo.txt');
readSync('foo.txt');"#;
    let result = analyze(code)?;
    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("fs.writeSync")).unwrap();
    assert_eq!(true, access.execute);
    let access = result.get(&JsWord::from("fs.readSync")).unwrap();
    assert_eq!(true, access.execute);
    Ok(())
}

#[test]
fn policy_builtin_access_read_var_init() -> Result<()> {
    let code = r#"const foo = process.env.FOO;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_assign() -> Result<()> {
    let code = r#"let foo; foo = process.env.FOO;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_paren() -> Result<()> {
    let code = r#"const foo = (process.env.FOO || '');"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_function_return() -> Result<()> {
    let code = r#"function foo() { return process.env.FOO; }"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_yield() -> Result<()> {
    let code = r#"function* foo() { yield process.env.FOO; }"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}
