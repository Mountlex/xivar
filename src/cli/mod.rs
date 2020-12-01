mod clean;
mod search;

use anyhow::Result;
use clean::Clean;
use search::Search;

use clap::Clap;

pub trait Command {
    fn run(&self) -> Result<()>;
}

#[derive(Clap)]
pub enum Cli {
    Search(Search),
    Clean(Clean),
}

impl Command for Cli {
    fn run(&self) -> Result<()> {
        match self {
            Cli::Search(cmd) => cmd.run(),
            Cli::Clean(cmd) => cmd.run(),
        }
    }
}
