use anyhow::Result;
use std::path::Path;

use swc_common::comments::SingleThreadedComments;
use swc_ecma_dep_graph::DependencyDescriptor;

use basalt::analysis::dependencies::analyze_dependencies;
use basalt::swc_utils::load_file;

fn load<P: AsRef<Path>>(file: P) -> Result<Vec<DependencyDescriptor>> {
    let (_file_name, source_map, module) = load_file(file)?;
    let comments: SingleThreadedComments = Default::default();
    Ok(analyze_dependencies(&source_map, &module, &comments)
        .builtins()
        .into_iter()
        .cloned()
        .collect())
}

#[test]
fn builtins_esm() -> Result<()> {
    let deps = load("tests/builtins/esm/input.js")?;
    assert_eq!(2, deps.len());
    assert_eq!("zlib", deps.get(0).unwrap().specifier.as_ref());
    assert_eq!("http", deps.get(1).unwrap().specifier.as_ref());
    Ok(())
}

#[test]
fn builtins_commonjs() -> Result<()> {
    let deps = load("tests/builtins/commonjs/input.js")?;
    assert_eq!(2, deps.len());
    assert_eq!("zlib", deps.get(0).unwrap().specifier.as_ref());
    assert_eq!("http", deps.get(1).unwrap().specifier.as_ref());
    Ok(())
}
