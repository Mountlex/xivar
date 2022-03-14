use anyhow::Result;
use clap::Parser;

use crate::{config, remotes::local::Library};

#[derive(Parser, Debug)]
#[clap(about = "Remove non-existent files from your library")]
pub struct Clean {
    #[clap(long)]
    all: bool,
}

impl Clean {
    pub fn run(&self) -> Result<()> {
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let removed = if self.all { lib.clear() } else { lib.clean() };

        if removed.is_empty() {
            println!("Nothing to remove.");
            Ok(())
        } else {
            for paper in removed {
                println!("Removed {:?}", paper.location);
            }
            lib.save()
        }
    }
}
