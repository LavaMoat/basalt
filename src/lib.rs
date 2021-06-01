use std::path::PathBuf;

use anyhow::{bail, Result};

mod bundler;
mod printer;
mod static_module_record;

pub use static_module_record::StaticModuleRecord;

pub fn list(
    entries: Vec<PathBuf>,
    include_file: bool,
    include_exports: bool,
) -> Result<()> {
    for f in entries.iter() {
        if !f.is_file() {
            bail!("Entry point {:?} does not exist", f);
        }
    }
    let options = printer::PrintOptions {
        print_tree: true,
        include_id: true,
        include_file,
        include_exports,
    };
    for f in entries.iter() {
        let printer = printer::Printer::new();
        printer.print(f, &options)?;
    }
    Ok(())
}
