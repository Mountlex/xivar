use super::Command;
use crate::config;
use crate::fzf;
use crate::remotes::dblp;
use crate::store::get_store_results;
use crate::store::{Library, MatchByTitle};
use crate::{
    arxiv::download_pdf,
    store::{Paper, PaperCopy},
};
use anyhow::Result;
use async_std::prelude::*;
use async_std::sync::{Arc, Mutex};
use async_std::task;
use clap::Clap;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};
use std::io::Write;
use std::{path::PathBuf, process::ChildStdin};

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
            async_find_and_write(async { get_store_results(&query, &lib) }, &handle_ref);
        let (store_results, online_results): (Vec<PaperCopy>, Vec<Paper>) = task::block_on(
            store_handle.try_join(dblp::fetch_publication_query(&query, self.num_hits)),
        )?;
        let new_results: Vec<Paper> = online_results
            .into_iter()
            .filter(|paper| !store_results.iter().any(|p| &p.paper == paper))
            .collect();
        task::block_on(async_write(&new_results, &handle_ref))?;

        let selected = fzf.wait_select()?;

        if let Some(paper_copy) = find_selection(&selected, &store_results) {
            if paper_copy.exists() {
                open::that(&paper_copy.location)?;
            } else {
                println!(
                    "The paper was not found at its expected location {:?}!",
                    paper_copy.location
                );
                lib.remove(&paper_copy.paper);
            }
        } else {
            if let Some(paper) = find_selection(&selected, &new_results) {
                if paper.url.preprint().is_some() {
                    let items = vec!["Download", "Open in Browser"];
                    let s = Select::with_theme(&ColorfulTheme::default())
                        .items(&items)
                        .default(0)
                        .interact_on_opt(&Term::stderr())?;
                    match s {
                        Some(0) => {
                            let dest = if let Some(output) = &self.output {
                                if output.is_file() {
                                    output.to_owned().with_extension("pdf")
                                } else {
                                    output.join(format!("{}.pdf", paper.clone().id))
                                }
                            } else {
                                config::xivar_document_dir()?
                                    .join(format!("{}.pdf", paper.clone().id))
                            };

                            let spinner = indicatif::ProgressBar::new_spinner();
                            spinner.set_style(
                                indicatif::ProgressStyle::default_spinner()
                                    .template("{msg} {spinner:.cyan/blue} "),
                            );
                            spinner.set_message("Downloading");
                            spinner.enable_steady_tick(10);
                            task::block_on(download_pdf(&paper.url.raw(), &dest))?;
                            spinner.abandon_with_message(&format!("Saved file to {:?}!", dest));
                            lib.add(&dest, paper);
                            open::that(dest)?;
                            lib.save()?;
                        }
                        Some(1) => {
                            open::that(paper.url.raw())?;
                        }
                        _ => println!("User did not select anything."),
                    }
                } else {
                    open::that(paper.url.raw())?;
                }
            }
        }

        Ok(())
    }
}

async fn async_find_and_write<F, P>(
    fetch: F,
    handle_ref: &Arc<Mutex<&mut ChildStdin>>,
) -> Result<Vec<P>>
where
    F: Future<Output = Result<Vec<P>>>,
    P: std::fmt::Display,
{
    let results = task::block_on(fetch)?;
    task::block_on(async_write(&results, handle_ref))?;
    Ok(results)
}

async fn async_write<P>(entries: &[P], handle_ref: &Arc<Mutex<&mut ChildStdin>>) -> Result<()>
where
    P: std::fmt::Display,
{
    for entry in entries {
        let mut handle = handle_ref.lock().await;
        if let Err(_) = writeln!(handle, "{}", entry) {
            // If this error occurs, fzf already closed, which is no problem.
        }
    }
    Ok(())
}

fn find_selection<P: MatchByTitle + Clone>(selection: &str, entries: &[P]) -> Option<P> {
    let selection: Vec<_> = selection.split("[").collect();
    let sel_title = selection.first().unwrap().to_owned();
    entries
        .into_iter()
        .find(|&paper| paper.matches_title(sel_title.trim()))
        .cloned()
}
