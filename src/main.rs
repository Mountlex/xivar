mod cli;
mod config;
mod fzf;
mod identifier;
mod paper;
mod query;
mod remotes;

pub use identifier::*;
pub use paper::*;
pub use query::Query;

use anyhow::Result;
use clap::Clap;
use cli::{Cli, Command};

fn main() -> Result<()> {
    Cli::parse().run()
}
