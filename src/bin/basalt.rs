use anyhow::Result;
use std::ffi::OsString;

fn main() -> Result<()> {
    basalt::cli::run::<OsString>(None)
}
