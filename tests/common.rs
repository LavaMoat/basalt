use anyhow::Result;
use std::path::Path;

pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut content = std::fs::read_to_string(path)?;
    if cfg!(target_os = "windows") {
        content = content.replace("\r\n", "\n");
    }
    Ok(content)
}

