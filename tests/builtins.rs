use anyhow::Result;
use std::path::Path;

use swc_common::comments::SingleThreadedComments;
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};

use basalt::analysis::dependencies::is_builtin_module;
use basalt::swc_utils::load_file;

fn load<P: AsRef<Path>>(file: P) -> Result<Vec<DependencyDescriptor>> {
    let (_file_name, source_map, module) = load_file(file)?;
    let comments: SingleThreadedComments = Default::default();
    Ok(analyze_dependencies(&module, &source_map, &comments))
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
    let deps = builtins(load("tests/builtins/esm/input.js")?);
    assert_eq!(2, deps.len());
    assert_eq!("zlib", deps.get(0).unwrap().specifier.as_ref());
    assert_eq!("http", deps.get(1).unwrap().specifier.as_ref());
    Ok(())
}

#[test]
fn builtins_commonjs() -> Result<()> {
    let deps = builtins(load("tests/builtins/commonjs/input.js")?);
    assert_eq!(2, deps.len());
    assert_eq!("zlib", deps.get(0).unwrap().specifier.as_ref());
    assert_eq!("http", deps.get(1).unwrap().specifier.as_ref());
    Ok(())
}
