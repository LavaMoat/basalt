use std::path::PathBuf;

use anyhow::{bail, Result};

mod resolver;

pub fn list(entries: Vec<PathBuf>) -> Result<()> {
    for f in entries.iter() {
        if !f.is_file() {
            bail!("Entry point {:?} does not exist", f);
        }
    }

    for f in entries.iter() {
        let resolver = resolver::Resolver::new();
        let entry = resolver.resolve(f)?;
        println!("{:?}", entry);
    }

    Ok(())
}
