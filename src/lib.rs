//! Analyzer, linter and bundler for LavaMoat.
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![deny(missing_docs)]

use std::path::PathBuf;

use anyhow::{bail, Result};
use swc::config::{JscTarget, SourceMapsConfig};

mod analysis;
mod module_node;
pub mod printer;
pub mod static_module_record;
mod swc_utils;

pub use static_module_record::{
    Generator, Parser,
};

/// List all modules.
pub fn list(file: PathBuf, include_file: bool) -> Result<()> {
    if !file.is_file() {
        bail!(
            "Module {} does not exist or is not a file",
            file.display()
        );
    }
    let options = printer::PrintOptions { include_file };
    let printer = printer::Printer::new();
    printer.print(file, &options)?;
    Ok(())
}

/// Print the static module record meta data as JSON.
pub fn meta(file: PathBuf) -> Result<()> {
    if !file.is_file() {
        bail!(
            "Module {} does not exist or is not a file",
            file.display()
        );
    }
    let parser = Parser::new();
    let (_, _, module) = crate::swc_utils::load_file(file)?;
    let smr = parser.parse(&module)?;
    let contents = serde_json::to_string_pretty(&smr)?;
    println!("{}", contents);
    Ok(())
}

/// Transform a module to a static module record program.
pub fn transform(file: PathBuf) -> Result<()> {
    if !file.is_file() {
        bail!(
            "Module {} does not exist or is not a file",
            file.display()
        );
    }
    let result = static_module_record::transform(file)?;
    print!("{}", result.code);
    Ok(())
}
