mod store;
mod config;
mod cli;

use anyhow::Result;
use cli::{Cli, Command};
use clap::Clap;

fn main() -> Result<()> {
    Cli::parse().run()
}
