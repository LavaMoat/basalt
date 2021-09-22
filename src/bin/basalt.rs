use std::ffi::OsString;
use anyhow::Result;

fn main() -> Result<()> {
    basalt::cli::run::<OsString>(None)
}
