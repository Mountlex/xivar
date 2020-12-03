use std::{future::Future, path::PathBuf, process::ChildStdin};

use anyhow::Result;
use async_std::{
    sync::{Arc, Mutex},
    task,
};
use std::io::Write;

use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};

use crate::{
    arxiv::download_pdf,
    config,
    store::{MatchByTitle, PaperCopy},
};

use crate::store::{Library, Paper};

pub fn select_remote_or_download(
    paper: Paper,
    lib: &mut Library,
    output: &Option<PathBuf>,
) -> Result<()> {
    if let Some(preprint) = paper.preprint() {
        let items = vec![
            format!("Download from {}", preprint.server_name()),
            "Open in Browser".to_owned(),
        ];
        match Select::with_theme(&ColorfulTheme::default())
            .items(&items)
            .default(0)
            .interact_on_opt(&Term::stderr())?
        {
            Some(0) => {
                let dest = if let Some(output) = output {
                    if output.is_file() {
                        output.to_owned().with_extension("pdf")
                    } else {
                        output.join(paper.default_filename())
                    }
                } else {
                    config::xivar_document_dir()?.join(paper.default_filename())
                }
                .with_extension("pdf");

                let spinner = indicatif::ProgressBar::new_spinner();
                spinner.set_style(
                    indicatif::ProgressStyle::default_spinner()
                        .template("{msg} {spinner:.cyan/blue} "),
                );
                spinner.set_message("Downloading");
                spinner.enable_steady_tick(10);
                task::block_on(download_pdf(&preprint.pdf_url().raw(), &dest))?;
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
    Ok(())
}

pub fn open_local_otherwise_download(
    paper: PaperCopy,
    lib: &mut Library,
    output: &Option<PathBuf>,
) -> Result<()> {
    if paper.exists() {
        open::that(paper.location)?;
    } else {
        println!("Paper is not located at old location! Do you want to");
        select_remote_or_download(paper.paper, lib, output)?;
    }
    Ok(())
}

pub async fn async_find_and_write<F, P>(
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

pub async fn async_write<P>(entries: &[P], handle_ref: &Arc<Mutex<&mut ChildStdin>>) -> Result<()>
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

pub fn find_selection<P: MatchByTitle + Clone>(selection: &str, entries: &[P]) -> Option<P> {
    let selection: Vec<_> = selection.split("[").collect();
    let sel_title = selection.first().unwrap().to_owned();
    entries
        .into_iter()
        .find(|&paper| paper.matches_title(sel_title.trim()))
        .cloned()
}
