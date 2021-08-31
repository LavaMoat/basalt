use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

use basalt::{bundle, globals, inspect, list, meta, parse, policy, transform};

#[derive(StructOpt)]
#[structopt(about = "Lavamoat analyzer and bundler")]
enum BasaltCommands {
    /// Print the module graph for an entry point
    Ls {
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
        /// Bundle entry point(s)
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

    /// Generate a policy file
    Policy {
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

fn main() -> Result<()> {
    if std::env::var("RUST_LOG").ok().is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let args = BasaltCommands::from_args();
    match args {
        BasaltCommands::Ls {
            module,
            include_file,
        } => {
            list(module, include_file)?;
        }
        BasaltCommands::Bundle { module, policy } => bundle(module, policy)?,
        BasaltCommands::Inspect { code, module } => inspect(code, module)?,
        BasaltCommands::Parse { module } => parse(module)?,
        BasaltCommands::Policy { module } => policy(module)?,
        BasaltCommands::Globals { module, debug } => globals(module, debug)?,
        BasaltCommands::Meta { module } => meta(module)?,
        BasaltCommands::Transform { module, json } => transform(module, json)?,
    }
    Ok(())
}
