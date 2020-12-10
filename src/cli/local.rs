use std::path::PathBuf;

use anyhow::Result;
use clap::Clap;
use fzf::Fzf;

use crate::store::{Library, Query};
use crate::{config, store::Paper};
use crate::{fzf, store::get_store_results};

use super::util;
use super::Command;

#[derive(Clap)]
pub struct Local {
    search_terms: Vec<String>,

    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[clap(short, long, default_value = "30")]
    num_hits: u32,
}

impl Command for Local {
    fn run(&self) -> Result<()> {
        let query = Query::builder()
            .terms(self.search_terms.clone())
            .max_hits(self.num_hits)
            .build();
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let mut fzf: Fzf<Paper> = Fzf::new()?;
        let results: Vec<Paper> = get_store_results(query, &lib);
        fzf.write_all(results);

        let paper = fzf.wait_for_selection()?;
        util::open_local_otherwise_download(paper, &mut lib, &self.output)
    }
}
