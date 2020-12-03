use std::path::PathBuf;

use super::{util, Command};
use crate::config;
use crate::fzf;
use crate::remotes::dblp;
use crate::store::get_store_results;
use crate::store::Library;
use crate::store::{Paper, PaperCopy};
use anyhow::{bail, Result};
use async_std::prelude::*;
use async_std::sync::{Arc, Mutex};
use async_std::task;
use clap::Clap;

#[derive(Clap, Debug)]
pub struct Search {
    query: Vec<String>,

    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[clap(short, long, default_value = "30")]
    num_hits: u32,
}

impl Command for Search {
    fn run(&self) -> Result<()> {
        let query = self.query.clone();
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let mut fzf = fzf::Fzf::new()?;
        let handle_ref = Arc::new(Mutex::new(fzf.stdin()));

        let store_handle =
            util::async_find_and_write(async { get_store_results(&query, &lib) }, &handle_ref);
        let (store_results, online_results): (Vec<PaperCopy>, Vec<Paper>) = task::block_on(
            store_handle.try_join(dblp::fetch_publication_query(&query, self.num_hits)),
        )?;
        let new_results: Vec<Paper> = online_results
            .into_iter()
            .filter(|paper| !store_results.iter().any(|p| &p.paper == paper))
            .collect();
        task::block_on(util::async_write(&new_results, &handle_ref))?;

        let selected = fzf.wait_select()?;

        if let Some(paper_copy) = util::find_selection(&selected, &store_results) {
            util::open_local_otherwise_download(paper_copy, &mut lib, &self.output)
        } else if let Some(paper) = util::find_selection(&selected, &new_results) {
            util::select_remote_or_download(paper, &mut lib, &self.output)
        } else {
            bail!("Internal error!")
        }
    }
}
