use std::io::Write;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Clap;

use crate::store::{Library, Query};
use crate::{config, store::PaperCopy};
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
            .terms(&self.search_terms)
            .max_hits(self.num_hits)
            .build();
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let mut fzf = fzf::Fzf::new()?;
        let handle = fzf.stdin();
        let results: Vec<PaperCopy> = get_store_results(&query, &lib);
        for result in results.iter() {
            writeln!(handle, "{}", result)?;
        }

        let selected = fzf.wait_select()?;

        if let Some(paper_copy) = util::find_selection(&selected, &results) {
            util::open_local_otherwise_download(paper_copy, &mut lib, &self.output)
        } else {
            bail!("Internal error!")
        }
    }
}
