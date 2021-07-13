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

    println!("Deps: {:#?}", deps);

    //let expected =
    //std::fs::read_to_string("tests/transform/reexport/output.js")?;
    //let (_, result) = transform(TransformSource::File(PathBuf::from(
    //"tests/builtins/exm/input.js",
    //)))?;
    //println!("{}", &result.code);
    //assert_eq!(expected, result.code);
    Ok(())
}
