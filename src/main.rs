//! Binary entrypoint for the `punch` CLI.

mod cli;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli::run(cli)
}
