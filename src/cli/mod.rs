mod actions;
mod add;
mod clean;
mod interactive;
mod local;
pub mod util;

use anyhow::Result;
use clean::Clean;
use local::Local;

use clap::{Parser, Subcommand};
pub use interactive::interactive;

#[derive(Parser, Debug)]
#[clap(
    version = "0.4.0",
    author = "Alexander Lindermayr <alexander.lindermayr97@gmail.com>",
    about = "Manage your local scientific library!"
)]
pub struct Cli {
    #[clap(subcommand)]
    helper: Option<Helpers>,
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        if let Some(helper) = &self.helper {
            helper.run()
        } else {
            interactive().await
        }
    }
}

#[derive(Subcommand, Debug)]
enum Helpers {
    Clean(Clean),
    Local(Local),
    Add(add::Add),
}

impl Helpers {
    fn run(&self) -> Result<()> {
        match &self {
            Helpers::Add(h) => h.run(),
            Helpers::Local(h) => h.run(),
            Helpers::Clean(h) => h.run(),
        }
    }
}
