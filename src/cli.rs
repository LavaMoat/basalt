//! Command line parsing exposed via the library for the node bindings.
use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

use super::{bundle, globals, inspect, tree, meta, parse, policy, transform};

#[derive(StructOpt)]
#[structopt(about = "Lavamoat analyzer and bundler")]
enum BasaltCommands {
    /// Print the module graph for an entry point
    Tree {
        /// Print the file name for each module
        #[structopt(short = "f", long)]
        include_file: bool,

        /// Module entry point
        #[structopt(parse(from_os_str))]
        module: PathBuf,
    },

    /// Generate a bundle
    Bundle {
        /// Path to policy file(s)
        #[structopt(short, long)]
        policy: Vec<PathBuf>,
        /// Source map destination
        #[structopt(short, long)]
        source_map: Option<PathBuf>,
        /// Source map URL
        #[structopt(short = "u", long)]
        source_map_url: Option<String>,
        /// Write bundle to output
        #[structopt(short, long)]
        output: Option<PathBuf>,
        /// Bundle entry point
        #[structopt(parse(from_os_str))]
        module: PathBuf,
    },

    /// Generate a policy file
    Policy {
        /// Module entry point
        #[structopt(parse(from_os_str))]
        module: PathBuf,
    },

    /// Print the AST for code or a file
    Inspect {
        /// Code to parse and print
        #[structopt(short, long)]
        code: Option<String>,
        /// Module entry point
        #[structopt(parse(from_os_str))]
        module: Option<PathBuf>,
    },

    /// Parse a dependency graph
    Parse {
        /// Module entry point
        #[structopt(parse(from_os_str))]
        module: PathBuf,
    },

    /// Print the globals for a module
    Globals {
        /// Print the scope hierarchy
        #[structopt(short, long)]
        debug: bool,

        /// Module entry point
        #[structopt(parse(from_os_str))]
        module: PathBuf,
    },

    /// Generate the static module record meta data for a module
    Meta {
        /// Module entry point
        #[structopt(parse(from_os_str))]
        module: PathBuf,
    },

    /// Transform a module to a functor program
    Transform {
        /// Print the meta data and program as JSON
        #[structopt(short, long)]
        json: bool,

        /// Module entry point
        #[structopt(parse(from_os_str))]
        module: PathBuf,
    },
}

/// Parse the given arguments list or `std::env::os_args` and run the program.
pub fn run<T>(argv: Option<Vec<T>>) -> Result<()>
where
    T: Into<OsString> + Clone,
{
    if std::env::var("RUST_LOG").ok().is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let app = BasaltCommands::clap();
    let matches = if let Some(argv) = argv {
        app.get_matches_from(argv)
    } else {
        app.get_matches()
    };
    let args = BasaltCommands::from_clap(&matches);
    match args {
        BasaltCommands::Tree {
            module,
            include_file,
        } => {
            tree(module, include_file)?;
        }
        BasaltCommands::Bundle {
            module,
            policy,
            output,
            source_map,
            source_map_url,
        } => bundle(module, policy, output, source_map, source_map_url)?,
        BasaltCommands::Inspect { code, module } => inspect(code, module)?,
        BasaltCommands::Parse { module } => parse(module)?,
        BasaltCommands::Policy { module } => policy(module)?,
        BasaltCommands::Globals { module, debug } => globals(module, debug)?,
        BasaltCommands::Meta { module } => meta(module)?,
        BasaltCommands::Transform { module, json } => transform(module, json)?,
    }
    Ok(())
}
