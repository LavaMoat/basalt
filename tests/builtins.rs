use anyhow::Result;
use std::path::Path;

use swc_common::comments::SingleThreadedComments;
use swc_ecma_dep_graph::DependencyDescriptor;

use basalt::analysis::dependencies::ModuleDependencyAnalysis;
use basalt::swc_utils::load_file;

fn load<P: AsRef<Path>>(file: P) -> Result<Vec<DependencyDescriptor>> {
    let (file_name, source_map, module) = load_file(file)?;
    let comments: SingleThreadedComments = Default::default();
    let analyzer = ModuleDependencyAnalysis::new(
        &file_name,
        &source_map,
        &module,
        &comments,
    );
    let deps: Vec<DependencyDescriptor> =
        analyzer.builtins().into_iter().cloned().collect();
    Ok(deps)
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
