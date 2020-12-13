use std::path::PathBuf;

use anyhow::Result;
use clap::Clap;

use crate::{
    config,
    fzf::Fzf,
    remotes::local::{Library, LocalPaper},
    Query,
};

use super::util;
use super::Command;

#[derive(Clap)]
#[clap(about = "Search your local library")]
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

        let mut fzf: Fzf<LocalPaper> = Fzf::new()?;
        let results: Vec<LocalPaper> = lib.iter_matches(&query).cloned().collect();

        fzf.write_all(results);

        let paper = fzf.wait_for_selection()?;
        util::open_local_otherwise_download(paper, &mut lib, self.output.as_ref())
    }
}
