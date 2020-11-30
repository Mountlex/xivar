use super::Command;
use crate::arxiv::{download_pdf, get_online_results};
use crate::config;
use crate::fzf;
use crate::store::get_store_results;
use crate::store::{Library, MatchByTitle};
use anyhow::{Context, Result};
use async_std::task;
use clap::Clap;
use dialoguer::{Confirm, Input};
use std::path::PathBuf;

use std::io::Write;

#[derive(Clap, Debug)]
pub struct Search {
    query: Vec<String>,

    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
}

impl Command for Search {
    fn run(&self) -> Result<()> {
        let query = self.query.clone();
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let results = task::block_on(get_store_results(&query, &lib))?;

        if results.is_empty()
            && Confirm::new()
                .with_prompt("Search results online?")
                .default(true)
                .interact()?
        {
            let online_results = task::block_on(get_online_results(query))?;

            let paper = open_fzf_and_select(&online_results)?;
            if Confirm::new()
                .with_prompt("Paper was not found on this machine. Download?")
                .default(true)
                .interact()?
            {
                let dest = if let Some(output) = &self.output {
                    if output.is_file() {
                        output.to_owned()
                    } else {
                        output.with_file_name(paper.id.as_str())
                    }
                } else {
                    config::xivar_document_dir()?.with_file_name(paper.id.as_str())
                }
                .with_extension("pdf");
                task::block_on(download_pdf(&paper, &dest))?;
                lib.add(dest, paper);
            }
        } else {
            let paper = open_fzf_and_select(&results)?;
        }
        lib.save()
    }
}

pub fn open_fzf_and_select<P: std::fmt::Display + MatchByTitle + Clone>(
    entries: &[P],
) -> Result<P> {
    let mut fzf = fzf::Fzf::new()?;
    let handle = fzf.stdin();
    for result in entries.iter() {
        writeln!(handle, "{}", result).context("Could not write to fzf!")?;
    }
    let sel = fzf.wait_select()?;
    let selection: Vec<_> = sel.split("[").collect();
    let sel_title = selection.first().unwrap().to_owned();
    let paper = entries
        .into_iter()
        .find(|&paper| paper.matches_title(sel_title.trim()))
        .cloned()
        .unwrap();
    Ok(paper)
}
