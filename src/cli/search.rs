use std::path::PathBuf;

use super::{util, Command};
use crate::fzf;
use crate::remotes;
use crate::{config, Query};

use anyhow::Result;
use async_std::task;
use clap::Clap;
use fzf::Fzf;
use indicatif::{ProgressBar, ProgressStyle};
use remotes::{local::Library, Paper};

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
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner().template("{msg:.bold} {spinner:.cyan/blue}"),
        );
        spinner.set_message("Searching");
        spinner.enable_steady_tick(10);

        let query = Query::builder()
            .terms(self.search_terms.clone())
            .max_hits(self.num_hits)
            .build();
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let papers = task::block_on(remotes::fetch_all_and_merge(&lib, query))?;
        spinner.finish_and_clear();

        let mut fzf: Fzf<Paper> = Fzf::new()?;
        fzf.write_all(papers);
        let paper = fzf.wait_for_selection()?;
        loop {
            let version = util::select_hit(paper.clone())?;
            if util::select_action_for_hit(version, &mut lib, self.output.as_ref()).is_ok() {
                break;
            }
        }
        Ok(())
    }
}
