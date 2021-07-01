use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

use basalt::{list, meta, symbols, transform};

#[derive(StructOpt)]
#[structopt(about = "Lavamoat analyzer and bundler")]
enum BasaltCommands {
    /// Print the module graph for entry point(s)
    Ls {
        /// Print the file name for each module
        #[structopt(short = "f", long)]
        include_file: bool,

        /// Module entry point
        #[structopt(parse(from_os_str))]
        module: PathBuf,
    },

    /// Print the symbols for a module
    Symbols {
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
        BasaltCommands::Symbols { module } => symbols(module)?,
        BasaltCommands::Meta { module } => meta(module)?,
        BasaltCommands::Transform { module, json } => transform(module, json)?,
    }
    Ok(())
}
