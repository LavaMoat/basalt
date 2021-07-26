//! Helper to determine the base path for a module.
use std::path::PathBuf;

/// Attempt to find the base directory for a module using the import specifier
/// and the resolved path for the module.
pub fn module_base_directory(
    specifier: &str,
    path: &PathBuf,
) -> Option<PathBuf> {
    // FIXME: refactor to walk looking for the nearest `package.json` file

    let mut sys_path = path.to_string_lossy().to_string();
    if cfg!(target_os = "windows") {
        sys_path = sys_path.replace("\\", "/");
    }
    let mut sub_path = vec!["node_modules"];
    let spec_parts: Vec<&str> = specifier.split("/").collect();
    if let Some(first) = spec_parts.get(0) {
        sub_path.push(first);
        if first.starts_with("@") {
            if let Some(second) = spec_parts.get(1) {
                sub_path.push(second);
            }
        }
    }
    let sub_path = sub_path.join("/");
    for i in sys_path.rmatch_indices(&sub_path).take(1) {
        let before = &sys_path[0..i.0];
        let mut pth = format!("{}{}", before, i.1);
        if cfg!(target_os = "windows") {
            pth = pth.replace("/", "\\");
        }
        return Some(PathBuf::from(pth));
    }

    None
}
