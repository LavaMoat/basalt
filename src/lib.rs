//! Analyzer, linter and bundler for LavaMoat.
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(once_cell)]
#![deny(missing_docs)]

use std::fs::OpenOptions;
use std::io::{self, prelude::*, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::{bail, Context, Result};

use swc::config::SourceMapsConfig;
use swc_common::SourceMap;
use swc_ecma_visit::VisitWith;

pub mod access;
pub mod bundler;
pub mod cli;
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

/// Write a file and create the parent directory when necessary.
fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(
    path: P,
    contents: C,
) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
        if !parent.is_dir() {
            bail!("target {} is not a directory", parent.display());
        }
    }
    std::fs::write(path, contents)?;
    Ok(())
}

/// Append sourceMappingURL to a target file.
fn append_source_mapping_url<P: AsRef<Path>>(path: P, url: &str) -> Result<()> {
    let url = format!("//# sourceMappingURL={}", url);
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path.as_ref())
        .unwrap();

    if let Err(e) = writeln!(file, "{}", &url) {
        bail!(
            "failed to append sourceMappingURL to file {}, reason: {}",
            path.as_ref().display(),
            e
        );
    }

    Ok(())
}

/// Generate a bundle.
pub fn bundle(
    module: PathBuf,
    policy: Vec<PathBuf>,
    output: Option<PathBuf>,
    source_map_path: Option<PathBuf>,
    source_map_url: Option<String>,
) -> Result<()> {
    if policy.is_empty() {
        bail!("bundle command requires some policy file(s) (use --policy)");
    }

    let module = module.canonicalize().context(format!(
        "unable to get canonical path for {}",
        module.display()
    ))?;

    let options = bundler::BundleOptions { module, policy };
    let (program, source_map) = bundler::bundle(options)?;
    let source_maps_config = SourceMapsConfig::Bool(true);
    let result =
        swc_utils::print(&program, source_map, None, None, source_maps_config)?;

    if let Some(path) = &output {
        write_file(path, result.code)?;
    } else {
        println!("{}", result.code);
    }

    // Write out the source map file
    if let (Some(path), Some(contents)) = (source_map_path, result.map) {
        write_file(&path, contents)?;

        // Handle appending sourceMappingURL to bundle file
        if let Some(output_path) = &output {
            let url = if let Some(url) = source_map_url {
                Some(url)
            } else {
                // FIXME: do not assume same directory by default,
                // FIXME: try to create relative path
                if let Some(file_name) = path.file_name() {
                    Some(file_name.to_string_lossy().into_owned())
                } else {
                    None
                }
            };

            if let Some(url) = &url {
                append_source_mapping_url(output_path, url)?;
            }
        }
    }

    Ok(())
}

/// Inspect the AST for a string or file.
pub fn inspect(code: Option<String>, file: Option<PathBuf>) -> Result<()> {
    if code.is_some() && file.is_some() {
        bail!(
            "the --code and file options are mutually exclusive, choose one."
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
        bail!("module {} does not exist or is not a file", file.display());
    }

    let builder = PolicyBuilder::new(file);
    let policy = builder.load()?.analyze()?.finalize();
    let policy_content = serde_json::to_string_pretty(&policy)?;
    println!("{}", policy_content);

    Ok(())
}

/// Print the dependency graph as a tree.
pub fn tree(file: PathBuf, include_file: bool) -> Result<()> {
    if !file.is_file() {
        bail!("module {} does not exist or is not a file", file.display());
    }
    let options = printer::PrintOptions { include_file };
    let printer = printer::Printer::new();
    printer.print(file, &options)?;
    Ok(())
}

/// Print the static module record meta data as JSON.
pub fn meta(file: PathBuf) -> Result<()> {
    if !file.is_file() {
        bail!("module {} does not exist or is not a file", file.display());
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
        bail!("module {} does not exist or is not a file", file.display());
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
        bail!("module {} does not exist or is not a file", file.display());
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

    let source_map: Arc<SourceMap> = Arc::new(Default::default());
    let (meta, result) = static_module_record::transform(source, source_map)?;
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
