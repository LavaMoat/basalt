//! Analyzer, linter and bundler for LavaMoat.
#![feature(unboxed_closures)]
#![feature(fn_traits)]

#![deny(missing_docs)]

use std::path::PathBuf;

use anyhow::{bail, Result};

mod analysis;
mod module_node;
pub mod printer;
pub mod static_module_record;
mod swc_utils;

pub use static_module_record::{Parser, StaticModuleRecord};

/// List all modules.
pub fn list(module: PathBuf, include_file: bool) -> Result<()> {
    if !module.is_file() {
        bail!(
            "Module {} does not exist or is not a file",
            module.display()
        );
    }
    let options = printer::PrintOptions { include_file };

    let printer = printer::Printer::new();
    printer.print(module, &options)?;
    Ok(())
}

/// Print the static module record as JSON.
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
