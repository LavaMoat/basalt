//! Analyzer, linter and bundler for LavaMoat.
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(once_cell)]
#![deny(missing_docs)]

use std::io::{self, Read};
use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::{bail, Context, Result};

use swc_ecma_visit::VisitWith;

pub mod access;
pub mod bundler;
pub mod helpers;
pub mod module;
pub mod policy;
pub mod printer;
pub mod static_module_record;
pub mod swc_utils;

pub use static_module_record::{
    Parser, StaticModuleRecordProgram, TransformSource,
};

use policy::{analysis::globals_scope::GlobalAnalysis, builder::PolicyBuilder};

/// Generate a bundle.
pub fn bundle(module: PathBuf, policy: Vec<PathBuf>) -> Result<()> {
    if policy.is_empty() {
        bail!("The bundle command requires some policy file(s) (use --policy)");
    }

    let module = module.canonicalize().context(format!(
        "unable to get canonical path for {}",
        module.display()
    ))?;

    let options = bundler::BundleOptions { module, policy };
    let (program, source_map) = bundler::bundle(options)?;
    let output = swc_utils::print(&program, source_map)?;

    //println!("{}", output.code);

    Ok(())
}

/// Inspect the AST for a string or file.
pub fn inspect(code: Option<String>, file: Option<PathBuf>) -> Result<()> {
    if code.is_some() && file.is_some() {
        bail!(
            "The --code and file options are mutually exclusive, choose one."
        );
    } else {
        if let Some(code) = code {
            let (_, _, module) = swc_utils::load_code(&code, None, None)?;
            println!("{:#?}", module);
        } else if let Some(file) = file {
            let (_, _, module) = swc_utils::load_file(&file, None)?;
            println!("{:#?}", module);
        }
    }
    Ok(())
}

/// Parse all the modules in a dependency graph.
pub fn parse(file: PathBuf) -> Result<()> {
    let now = SystemTime::now();
    let (parsed_modules, visited_modules) = module::parser::parse(file)?;
    if let Ok(t) = now.elapsed() {
        log::debug!("Visited {} module(s)", visited_modules);
        log::info!("Parsed {} module(s) in {:?}", parsed_modules, t);
    }
    Ok(())
}

/// Generate a policy file.
pub fn policy(file: PathBuf) -> Result<()> {
    if !file.is_file() {
        bail!("Module {} does not exist or is not a file", file.display());
    }

    let builder = PolicyBuilder::new(file);
    let policy = builder.load()?.analyze()?.finalize();
    let policy_content = serde_json::to_string_pretty(&policy)?;
    println!("{}", policy_content);

    Ok(())
}

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
    let (_, _, module) = crate::swc_utils::load_file(file, None)?;
    let smr = parser.parse(&module)?;
    let contents = serde_json::to_string_pretty(&smr)?;
    println!("{}", contents);
    Ok(())
}

/// Print the globals in a module.
///
/// By default it prints the global symbols in a module, if the
/// debug option is given the scope tree is printed.
pub fn globals(file: PathBuf, debug: bool) -> Result<()> {
    if !file.is_file() {
        bail!("Module {} does not exist or is not a file", file.display());
    }

    let mut analyzer = GlobalAnalysis::new(Default::default());
    let (_, _, module) = crate::swc_utils::load_file(&file, None)?;
    module.visit_children_with(&mut analyzer);

    if debug {
        println!("{:#?}", analyzer);
    } else {
        let globals = analyzer.compute_globals();
        let globals = analyzer.flatten_join(globals);
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

/// Helper to remove a trailing semi-colon from the generated functor program
/// as a trailing semi-colon breaks static module record interoperability.
fn trim_code(code: String) -> String {
    let out = code.trim_end();
    out.trim_end_matches(";").to_string()
}
