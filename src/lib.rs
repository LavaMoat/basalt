use std::path::PathBuf;

use anyhow::{bail, Result};

mod bundler;
mod printer;

pub fn list(entries: Vec<PathBuf>) -> Result<()> {
    for f in entries.iter() {
        if !f.is_file() {
            bail!("Entry point {:?} does not exist", f);
        }
    }
    let options = printer::PrintOptions {
        print_tree: true,
        include_id: true,
        include_file: false,
    };
    for f in entries.iter() {
        let printer = printer::Printer::new();
        printer.print(f, &options)?;
    }
    Ok(())
}
