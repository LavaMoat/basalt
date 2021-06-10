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

pub use static_module_record::{Generator, Parser, StaticModuleRecord};

/// Operations for static module record generation.
pub enum StaticModuleRecordOperation {
    /// Generate meta data
    Meta,
    /// Generate the functor program
    Functor,
}

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
pub fn smr(module: PathBuf, op: StaticModuleRecordOperation) -> Result<()> {
    if !module.is_file() {
        bail!(
            "Module {} does not exist or is not a file",
            module.display()
        );
    }
    let parser = Parser::new();
    let smr = parser.load(module)?;

    match op {
        StaticModuleRecordOperation::Meta => {
            let contents = serde_json::to_string_pretty(&smr)?;
            println!("{}", contents);
        }
        StaticModuleRecordOperation::Functor => {
            let generator = Generator::new(&smr);
            let script = generator.create()?;
            let (_source_map, compiler) = crate::swc_utils::get_compiler();
            let result = compiler.print(
                &script,
                JscTarget::Es2020,
                SourceMapsConfig::Bool(true),
                None,
                false,
            )?;
            //println!("{:#?}", result);
            print!("{}", result.code);
        }
    }
    Ok(())
}
