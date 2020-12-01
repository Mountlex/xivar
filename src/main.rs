mod arxiv;
mod cli;
mod config;
mod fzf;
mod store;

use anyhow::Result;
use clap::Clap;
use cli::{Cli, Command};

fn main() -> Result<()> {
    Cli::parse().run()
}
