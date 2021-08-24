use anyhow::Result;

use swc_common::comments::SingleThreadedComments;
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};

use basalt::module::dependencies::is_builtin_module;
use basalt::swc_utils::load_code;

fn load(code: &str) -> Result<Vec<DependencyDescriptor>> {
    let (_file_name, _source_map, module) = load_code(code, None)?;
    let comments: SingleThreadedComments = Default::default();
    Ok(analyze_dependencies(&module, &comments))
}

fn builtins(deps: Vec<DependencyDescriptor>) -> Vec<DependencyDescriptor> {
    deps.into_iter()
        .filter_map(|dep| {
            if is_builtin_module(dep.specifier.as_ref()) {
                Some(dep)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn builtins_esm() -> Result<()> {
    let code = r#"
        import * as foo from './foo';
        import {a, b, c} from './abc.js';
        import zlib from 'zlib';
        import React from 'react';
        import http from 'http';"#;
    let deps = builtins(load(code)?);

    assert_eq!(2, deps.len());
    assert_eq!("zlib", deps.get(0).unwrap().specifier.as_ref());
    assert_eq!("http", deps.get(1).unwrap().specifier.as_ref());
    Ok(())
}

#[test]
fn builtins_commonjs() -> Result<()> {
    let code = r#"
        const foo = require('./foo');
        const {a, b, c} = require('./abc.js');
        const zlib = require('zlib');
        const React = require('react');
        const http = require('http');"#;
    let deps = builtins(load(code)?);
    assert_eq!(2, deps.len());
    assert_eq!("zlib", deps.get(0).unwrap().specifier.as_ref());
    assert_eq!("http", deps.get(1).unwrap().specifier.as_ref());
    Ok(())
}
