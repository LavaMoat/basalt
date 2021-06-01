use std::path::PathBuf;

use anyhow::{bail, Result};

mod bundler;
mod printer;
mod static_module_record;
mod utils;

pub use static_module_record::{StaticModuleRecord, Parser};

pub fn list(
    entries: Vec<PathBuf>,
    include_file: bool,
    include_exports: bool,
) -> Result<()> {
    for f in entries.iter() {
        if !f.is_file() {
            bail!("Entry point {} does not exist or is not a file", f.display());
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

pub fn smr(
    module: PathBuf,
) -> Result<()> {
    if !module.is_file() {
        bail!("Module {} does not exist or is not a file", module.display());
    }
    let parser = Parser::new();
    let smr = parser.load(module)?;
    let contents = serde_json::to_string_pretty(&smr)?;
    println!("{}", contents);
    Ok(())
}
