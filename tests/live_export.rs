use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use swc_common::SourceMap;

use basalt::static_module_record::{transform, TransformSource};

#[test]
fn live_export_assignment() -> Result<()> {
    let source_map: Arc<SourceMap> = Arc::new(Default::default());
    let expected = std::fs::read_to_string(
        "tests/transform/live-export-assignment/output.js",
    )?;
    let (_, result) = transform(
        TransformSource::File(PathBuf::from(
            "tests/transform/live-export-assignment/input.js",
        )),
        source_map,
    )?;
    //println!("{}", &result.code);
    assert_eq!(expected, result.code);
    Ok(())
}
