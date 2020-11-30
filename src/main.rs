mod store;
mod config;
mod cli;
mod arxiv;
mod fzf;

use anyhow::Result;
use cli::{Cli, Command};
use clap::Clap;


fn main() -> Result<()> {
    Cli::parse().run()
}
