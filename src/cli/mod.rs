mod search;

use anyhow::Result;
use search::Search;

use clap::Clap;

pub trait Command {
    fn run(&self) -> Result<()>;
}

#[derive(Clap)]
pub enum Cli {
    Search(Search),
}

impl Command for Cli {
    fn run(&self) -> Result<()> {
        match self {
            Cli::Search(cmd) => cmd.run(),
        }
    }
}