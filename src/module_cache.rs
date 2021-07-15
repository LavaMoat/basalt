//! Loads modules and caches them using the file path as the key.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use swc_common::{FileName, SourceMap};
use swc_ecma_ast::Module;

use dashmap::DashMap;
use std::lazy::SyncLazy;

use crate::swc_utils::load_file;

static CACHE: SyncLazy<
    DashMap<PathBuf, (Arc<Module>, Arc<SourceMap>, Arc<FileName>)>,
> = SyncLazy::new(|| DashMap::new());

/// Load and parse a module or retrieve a module from the cache.
pub fn load_module<P: AsRef<Path>>(
    file: P,
) -> Result<(Arc<Module>, Arc<SourceMap>, Arc<FileName>)> {
    let buf = file.as_ref().to_path_buf();
    if let Some(entry) = CACHE.get(&buf) {
        let (module, source_map, file_name) = entry.value();
        return Ok((module.clone(), source_map.clone(), file_name.clone()))
    }
    let (file_name, source_map, module) = load_file(file.as_ref())?;
    let entry = CACHE
        .entry(buf)
        .or_insert((
            Arc::new(module),
            source_map.clone(),
            Arc::new(file_name),
        ));

    let (module, source_map, file_name) = entry.value();
    Ok((module.clone(), source_map.clone(), file_name.clone()))
}
