//! Analyzer, linter and bundler for LavaMoat.
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![deny(missing_docs)]

use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{bail, Result};

use swc_ecma_visit::VisitWith;

pub mod analysis;
pub mod helpers;
mod module_node;
pub mod printer;
pub mod static_module_record;
pub mod swc_utils;

pub use static_module_record::{
    Parser, StaticModuleRecordProgram, TransformSource,
};

use analysis::globals_scope::ScopeAnalysis;

/// List all modules.
pub fn list(file: PathBuf, include_file: bool) -> Result<()> {
    if !file.is_file() {
        bail!("Module {} does not exist or is not a file", file.display());
    }
    let options = printer::PrintOptions { include_file };
    let printer = printer::Printer::new();
    printer.print(file, &options)?;
    Ok(())
}

/// Print the static module record meta data as JSON.
pub fn meta(file: PathBuf) -> Result<()> {
    if !file.is_file() {
        bail!("Module {} does not exist or is not a file", file.display());
    }
    let mut parser = Parser::new();
    let (_, _, module) = crate::swc_utils::load_file(file)?;
    let smr = parser.parse(&module)?;
    let contents = serde_json::to_string_pretty(&smr)?;
    println!("{}", contents);
    Ok(())
}

/// Print the symbols in a module.
///
/// By default it prints the global symbols in a module, if the
/// debug option is given the scope tree is printed.
pub fn symbols(file: PathBuf, debug: bool) -> Result<()> {
    if !file.is_file() {
        bail!("Module {} does not exist or is not a file", file.display());
    }

    let mut block_scope = ScopeAnalysis::new();
    let (_, _, module) = crate::swc_utils::load_file(&file)?;
    module.visit_children_with(&mut block_scope);

    if debug {
        println!("{:#?}", block_scope);
    } else {
        let globals = block_scope.globals();
        println!("{}", serde_json::to_string_pretty(&globals)?);
    }

    Ok(())
}

/// Transform a module to a static module record program.
pub fn transform(file: PathBuf, json: bool) -> Result<()> {
    let is_stdin = PathBuf::from("-") == file;
    if !file.is_file() && !is_stdin {
        bail!("Module {} does not exist or is not a file", file.display());
    }

    let source = if is_stdin {
        let mut buffer = String::new();
        let mut stdin = io::stdin();
        stdin.read_to_string(&mut buffer)?;
        TransformSource::Str {
            content: buffer,
            file_name: String::from("stdin"),
        }
    } else {
        TransformSource::File(file)
    };

    let (meta, result) = static_module_record::transform(source)?;
    if json {
        let output = StaticModuleRecordProgram {
            meta,
            program: trim_code(result.code),
        };
        print!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print!("{}", result.code);
    }
    Ok(())
}

fn trim_code(code: String) -> String {
    let out = code.trim_end();
    out.trim_end_matches(";").to_string()
}
