use anyhow::Result;
use clap::Parser;

use crate::{library::Library, xiv_config::Config};

#[derive(Parser, Debug)]
#[clap(about = "Remove non-existent files from your library")]
pub struct Clean {
    #[clap(long)]
    all: bool,
}

impl Clean {
    pub fn run(&self, config: Config) -> Result<()> {
        let data_dir = config.data_dir;
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
