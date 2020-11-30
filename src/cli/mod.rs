mod query;

use anyhow::Result;
use query::Query;

use clap::Clap;

pub trait Command {
    fn run(&self) -> Result<()>;
}

#[derive(Clap)]
pub enum Cli {
    Query(Query)
}

impl Command for Cli {
    fn run(&self) -> Result<()> {
        match self {
            Cli::Query(cmd) => cmd.run(),
        }
    }
}