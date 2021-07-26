//! Helper to determine the base path for a module.
use std::path::PathBuf;

const PACKAGE: &str = "package.json";

/// Attempt to find the base directory for a module using the resolved path for the module.
pub fn module_base_directory(path: &PathBuf) -> Option<PathBuf> {
    let mut parent = path.parent();
    while let Some(p) = parent {
        let pkg = p.join(PACKAGE);
        if pkg.is_file() {
            return Some(p.to_path_buf());
        }
        parent = p.parent();
    }
    None
}
