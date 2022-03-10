use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::{config, finder, remotes::local::Library, Query};

use super::util;
use super::Command;

#[derive(Parser, Debug)]
#[clap(about = "Search your local library")]
pub struct Local {
    search_terms: Vec<String>,

    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[clap(short, long)]
    num_hits: Option<u32>,
}

impl Command for Local {
    fn run(&self) -> Result<()> {
        let query = Query::builder()
            .terms(self.search_terms.clone())
            .max_hits(self.num_hits)
            .build();
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let result_iter = lib.iter_matches(&query).cloned();

        let paper = finder::show_and_select(result_iter)?;
        util::open_local_otherwise_download(paper, &mut lib, self.output.as_ref())
    }
}
