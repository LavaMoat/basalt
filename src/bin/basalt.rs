use std::path::PathBuf;

use anyhow::{bail, Result};
use structopt::StructOpt;

use basalt::list;

#[derive(StructOpt)]
#[structopt(about = "Lavamoat analyzer and bundler")]
enum BasaltCommands {
    /// List imports for entry points
    Ls {
        #[structopt(parse(from_os_str))]
        entries: Vec<PathBuf>,
    },
}

fn main() -> Result<()> {
    if std::env::var("RUST_LOG").ok().is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let args = BasaltCommands::from_args();
    match args {
        BasaltCommands::Ls { entries } => {
            if entries.is_empty() {
                bail!("List command requires entry points.");
            }
            list(entries)?;
        }
    }
    Ok(())
}
