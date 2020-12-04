use crate::store::Library;
use anyhow::Result;
use clap::Clap;

use super::Command;
use crate::config;

#[derive(Clap, Debug)]
pub struct Clean {}

impl Command for Clean {
    fn run(&self) -> Result<()> {
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let removed = lib.clean();
        for paper in removed {
            println!("Removed {:?}", paper.local_path);
        }

        lib.save()
    }
}
