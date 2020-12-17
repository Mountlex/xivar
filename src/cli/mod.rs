mod actions;
mod add;
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
#[clap(
    version = "0.4.0",
    author = "Alexander Lindermayr <alexander.lindermayr97@gmail.com>",
    about = "Manage your local scientific library!"
)]
pub enum Cli {
    Search(Search),
    Clean(Clean),
    Local(Local),
    Add(add::Add),
}

impl Command for Cli {
    fn run(&self) -> Result<()> {
        match self {
            Cli::Search(cmd) => cmd.run(),
            Cli::Clean(cmd) => cmd.run(),
            Cli::Local(cmd) => cmd.run(),
            Cli::Add(cmd) => cmd.run(),
        }
    }
}
