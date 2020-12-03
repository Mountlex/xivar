use std::io::Write;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Clap;

use crate::fzf;
use crate::store::Library;
use crate::{config, store::PaperCopy};

use super::util;
use super::Command;

#[derive(Clap)]
pub struct Local {
    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
}

impl Command for Local {
    fn run(&self) -> Result<()> {
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let mut fzf = fzf::Fzf::new()?;
        let handle = fzf.stdin();
        let results: Vec<PaperCopy> = lib.iter_all().cloned().collect();
        for result in results.iter() {
            write!(handle, "{}", result)?;
        }

        let selected = fzf.wait_select()?;

        if let Some(paper_copy) = util::find_selection(&selected, &results) {
            util::open_local_otherwise_download(paper_copy, &mut lib, &self.output)
        } else {
            bail!("Internal error!")
        }
    }
}
