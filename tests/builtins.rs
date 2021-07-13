use anyhow::Result;
use std::path::Path;

use basalt::analysis::builtins::analyze;
use basalt::swc_utils::load_file;

use swc_ecma_dep_graph::DependencyDescriptor;

fn load<P: AsRef<Path>>(file: P) -> Result<Vec<DependencyDescriptor>> {
    let (_file_name, source_map, module) = load_file(file)?;
    Ok(analyze(&module, &source_map, Default::default()))
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
