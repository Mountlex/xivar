use std::path::PathBuf;

use super::{util, Command};
use crate::config;
use crate::fzf;
use crate::remotes;
use crate::store::get_store_results;
use crate::store::Library;
use crate::store::Query;
use anyhow::Result;
use async_std::prelude::*;
use async_std::task;
use clap::Clap;

#[derive(Clap, Debug)]
pub struct Search {
    search_terms: Vec<String>,

    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[clap(short, long, default_value = "100")]
    num_hits: u32,
}

impl Command for Search {
    fn run(&self) -> Result<()> {
        let query = Query::builder()
            .terms(self.search_terms.clone())
            .max_hits(self.num_hits)
            .build();
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let fzf = fzf::Fzf::new()?;

        let store_handle =
            fzf.fetch_and_write(async { Ok(get_store_results(query.clone(), &lib)) });
        let online_handle = fzf.fetch_and_write(remotes::fetch_all_and_merge(query.clone()));
        task::block_on(store_handle.try_join(online_handle))?;

        let paper = fzf.wait_for_selection()?;

        if paper.local_path.is_some() {
            util::open_local_otherwise_download(paper, &mut lib, &self.output)
        } else {
            util::select_remote_or_download(paper, &mut lib, &self.output)
        }
    }
}
