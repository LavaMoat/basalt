use std::path::PathBuf;

use anyhow::{bail, Result};

mod imports;
mod printer;
mod static_module_record;
mod swc_utils;
mod types;

pub use static_module_record::{Parser, StaticModuleRecord};

pub fn list(module: PathBuf, include_file: bool) -> Result<()> {
    if !module.is_file() {
        bail!(
            "Module {} does not exist or is not a file",
            module.display()
        );
    }
    let options = printer::PrintOptions {
        print_tree: true,
        include_file,
    };

    let printer = printer::Printer::new();
    printer.print(module, &options)?;
    Ok(())
}

pub fn smr(module: PathBuf) -> Result<()> {
    if !module.is_file() {
        bail!(
            "Module {} does not exist or is not a file",
            module.display()
        );
    }
    let parser = Parser::new();
    let smr = parser.load(module)?;
    let contents = serde_json::to_string_pretty(&smr)?;
    println!("{}", contents);
    Ok(())
}
