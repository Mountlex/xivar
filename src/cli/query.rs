use anyhow::Result;
use crate::config;
use crate::store::{Library};

use clap::Clap;
use super::Command;

#[derive(Clap, Debug)]
pub struct Query {
    query: Vec<String>
}

impl Command for Query {
    fn run(&self) -> Result<()> {
        let data_dir = config::xivar_data_dir()?;
        let lib = Library::open(&data_dir)?;
        Ok(())
    }
}