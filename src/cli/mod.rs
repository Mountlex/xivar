mod clean;
mod local;
mod search;
pub mod util;

use anyhow::Result;
use clean::Clean;
use local::Local;
use search::Search;

use clap::Clap;

pub trait Command {
    fn run(&self) -> Result<()>;
}

#[derive(Clap)]
pub enum Cli {
    Search(Search),
    Clean(Clean),
    Local(Local),
}

impl Command for Cli {
    fn run(&self) -> Result<()> {
        match self {
            Cli::Search(cmd) => cmd.run(),
            Cli::Clean(cmd) => cmd.run(),
            Cli::Local(cmd) => cmd.run(),
        }
    }
}
