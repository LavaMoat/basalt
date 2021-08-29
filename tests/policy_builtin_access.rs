use anyhow::Result;
use indexmap::IndexMap;
use swc_atoms::JsWord;
use swc_ecma_visit::VisitWith;

use basalt::{
    access::Access,
    policy::analysis::{
        builtin::BuiltinAnalysis, globals_scope::GlobalAnalysis,
    },
    swc_utils::load_code,
};

fn analyze(code: &str) -> Result<IndexMap<JsWord, Access>> {
    let (_, _, module) = load_code(code, None, None)?;

    let mut globals_scope = GlobalAnalysis::new(Default::default());
    module.visit_children_with(&mut globals_scope);
    let builtin_candidates =
        std::mem::take(&mut globals_scope.builder.candidates);

    let analyzer: BuiltinAnalysis = Default::default();
    Ok(analyzer.analyze(&module, builtin_candidates))
}

// WRITE

#[test]
fn policy_builtin_access_write() -> Result<()> {
    let code = r#"
        import process from 'process';
        process.env.FOO = 1;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.write);
    Ok(())
}

#[test]
fn policy_builtin_access_write_update() -> Result<()> {
    let code = r#"
        import process from 'process';
        process.env.FOO++;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.write);
    Ok(())
}

// EXECUTE

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
fn policy_builtin_access_execute_await() -> Result<()> {
    let code = r#"import {readFile} from 'fs/promises'; async function foo() { await readFile('foo.txt') }"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("fs/promises.readFile")).unwrap();
    assert_eq!(true, access.execute);
    Ok(())
}

#[test]
fn policy_builtin_access_execute_function_expression() -> Result<()> {
    let code = r#"
        var fs = require("fs");
        function ZipFile() {}
        // Function expression recursion ->>>
        ZipFile.prototype.addFile = function(realPath) {
          fs.stat(realPath, function(err, stats) {});
        }
        "#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("fs.stat")).unwrap();
    assert_eq!(true, access.execute);
    Ok(())
}

#[test]
fn policy_builtin_access_execute_require_dot_member_expr() -> Result<()> {
    let code = r#"
        var TransformStream = require("stream").Transform;
        var util = require("util");
        util.inherits(ByteCounter, TransformStream);
        function ByteCounter(options) {
          TransformStream.call(this, options);
        }
        "#;
    let result = analyze(code)?;

    println!("Result {:#?}", result);

    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("util.inherits")).unwrap();
    assert_eq!(true, access.execute);
    let access = result.get(&JsWord::from("stream.Transform")).unwrap();
    assert_eq!(true, access.execute);
    Ok(())
}

// READ

#[test]
fn policy_builtin_access_read_var_init() -> Result<()> {
    let code = r#"
        import process from 'process';
        const foo = process.env.FOO;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_assign() -> Result<()> {
    let code = r#"
        import process from 'process';
        let foo; foo = process.env.FOO;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_paren() -> Result<()> {
    let code = r#"
        import process from 'process';
        const foo = (process.env.FOO || '');"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_function_return() -> Result<()> {
    let code = r#"
        import process from 'process';
        function foo() { return process.env.FOO; }"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_yield() -> Result<()> {
    let code = r#"
        import process from 'process';
        function* foo() { yield process.env.FOO; }"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_unary() -> Result<()> {
    let code = r#"
        import process from 'process';
        const foo = typeof process.env.FOO;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_bin() -> Result<()> {
    let code = r#"
        import process from 'process';
        const equals = process.env.FOO !== process.env.BAR;"#;
    let result = analyze(code)?;
    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAR")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_cond() -> Result<()> {
    let code = r#"
        import process from 'process';
        process.env.FOO ? process.env.BAR : process.env.QUX;"#;
    let result = analyze(code)?;
    assert_eq!(3, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAR")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.QUX")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_tpl() -> Result<()> {
    let code = r#"
        import process from 'process';
        const msg = `${process.env.FOO}`"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_tagged_tpl() -> Result<()> {
    let code = r#"
        import process from 'process';
        const msg = html`${process.env.FOO}`"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_block_stmt() -> Result<()> {
    let code = r#"
        import process from 'process';
        { const foo = process.env.FOO; }"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_with() -> Result<()> {
    let code = r#"
        import process from 'process';
        with(process) { const foo = env.FOO }"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_if() -> Result<()> {
    let code = r#"
        import process from 'process';
        if(process.env.FOO) { const bar = process.env.BAR; }"#;
    let result = analyze(code)?;
    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAR")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_switch() -> Result<()> {
    let code = r#"
        import process from 'process';
        switch(process.env.FOO) { case process.env.BAR: break; }"#;
    let result = analyze(code)?;
    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAR")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_throw() -> Result<()> {
    let code = r#"
        import process from 'process';
        throw process.env.FOO;"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_try() -> Result<()> {
    let code = r#"
        import process from 'process';
        try { const foo = process.env.FOO; }
        catch { const bar = process.env.BAR; }
        finally { const qux = process.env.QUX; }"#;
    let result = analyze(code)?;
    assert_eq!(3, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAR")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.QUX")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_while() -> Result<()> {
    let code = r#"
        import process from 'process';
        while(process.env.FOO !== '') { const bar = process.env.BAR };"#;
    let result = analyze(code)?;
    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAR")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_for() -> Result<()> {
    let code = r#"
        import process from 'process';
        for(let i = parseInt(process.env.FOO); i < parseInt(process.env.BAR);i++) {}"#;
    let result = analyze(code)?;
    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAR")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_for_in() -> Result<()> {
    let code = r#"
        import process from 'process';
        for(let i in process.env.FOO) {}"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_for_of() -> Result<()> {
    let code = r#"
        import process from 'process';
        for(const i of process.env.FOO) {}"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_class_prop() -> Result<()> {
    let code = r#"
        import process from 'process';
        class Foo { prop = process.env.FOO; }"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_class_private_prop() -> Result<()> {
    let code = r#"
        import process from 'process';
        class Foo { #prop = process.env.FOO; }"#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_class_constructor() -> Result<()> {
    let code = r#"
        import process from 'process';
        class FooBar {
            constructor(prop = process.env.FOO) {
                const bar = process.env.BAR;
            }
        }"#;
    let result = analyze(code)?;
    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAR")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_class_method() -> Result<()> {
    let code = r#"
        import process from 'process';
        class FooBar {
            doSomething(prop = process.env.FOO) {
                const bar = process.env.BAR;
            }

            #doSomethingPrivate(prop = process.env.BAZ) {
                const bar = process.env.QUX;
            }

        }"#;
    let result = analyze(code)?;
    assert_eq!(4, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAR")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.BAZ")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("process.env.QUX")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_require_dot_access() -> Result<()> {
    let code = r#"
        var PassThrough = require("stream").PassThrough;
        var EventEmitter = require("events").EventEmitter;
        util.inherits(ZipFile, EventEmitter);
        function ZipFile() {
          this.outputStream = new PassThrough();
        }
        "#;
    let result = analyze(code)?;
    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("events.EventEmitter")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("stream.PassThrough")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_arrow() -> Result<()> {
    let code = r#"
        import fs from 'fs';
        const foo = (stat = fs.stat) => {
            const statSync = fs.statSync;
        }
        "#;
    let result = analyze(code)?;
    assert_eq!(2, result.len());
    let access = result.get(&JsWord::from("fs.stat")).unwrap();
    assert_eq!(true, access.read);
    let access = result.get(&JsWord::from("fs.statSync")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

#[test]
fn policy_builtin_access_read_default_import_rename() -> Result<()> {
    let code = r#"
        import ps from 'process';
        const foo = ps.env.FOO;
        "#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("process.env.FOO")).unwrap();
    assert_eq!(true, access.read);
    Ok(())
}

// MISC

#[test]
fn policy_builtin_access_merge() -> Result<()> {
    let code = r#"
        import buffer from 'buffer';
        // Read access
        const buf1 = buffer.Buffer;
        // Execute to a nested path
        const buf2 = buffer.Buffer.from([]);
        "#;
    let result = analyze(code)?;
    assert_eq!(1, result.len());
    let access = result.get(&JsWord::from("buffer.Buffer")).unwrap();
    assert_eq!(true, access.read);
    assert_eq!(true, access.execute);
    Ok(())
}
