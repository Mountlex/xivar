mod cli;
mod config;
mod finder;
mod identifier;
mod paper;
mod query;
mod remotes;

pub use identifier::*;
pub use paper::*;
pub use query::Query;

use clap::Clap;
use cli::{Cli, Command};

fn main() {
    if let Err(error) = Cli::parse().run() {
        println!("{}", error);
    }
}
