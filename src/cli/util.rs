use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use async_std::task;
use remotes::{
    local::{Library, LocalPaper},
    Paper, PaperHit, RemoteTag,
};

use std::io::Write;

use console::style;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};

use crate::{config, fzf::Fzf, remotes, PaperInfo, PaperUrl, Query};

pub fn select_hit(paper: Paper) -> Result<PaperHit> {
    let hits = paper.hits();
    if hits.len() <= 1 {
        hits.first().cloned().ok_or(anyhow!("No paper given!"))
    } else {
        let items = hits
            .iter()
            .map(|hit| format!("{}", hit.remote_tag()))
            .collect::<Vec<String>>();
        match Select::with_theme(&ColorfulTheme::default())
            .items(&items)
            .with_prompt("Select version")
            .default(0)
            .interact_on_opt(&Term::stderr())?
        {
            Some(i) => {
                if i < items.len() {
                    Ok(hits[i].clone())
                } else {
                    bail!("Internal error!")
                }
            }
            _ => bail!("User did not select any remote! Aborting!"),
        }
    }
}

pub fn select_action_for_hit(
    paper: PaperHit,
    lib: &mut Library,
    output: Option<&PathBuf>,
) -> Result<()> {
    match paper {
        PaperHit::Local(paper) => {
            open::that(paper.location)?;
        }
        PaperHit::Dblp(paper) => {
            let urls = vec![paper.ee.raw(), paper.url.raw()];
            match Select::with_theme(&ColorfulTheme::default())
                .items(&urls)
                .default(0)
                .with_prompt("Select reference")
                .interact_on_opt(&Term::stderr())?
            {
                Some(i) => {
                    if i < urls.len() {
                        open::that(urls[i].clone())?;
                    } else {
                        bail!("Internal error!");
                    }
                }
                _ => {
                    bail!("User did not select any remote! Aborting!");
                }
            }
        }
        PaperHit::Arxiv(paper) => {
            match Select::with_theme(&ColorfulTheme::default())
                .items(&vec!["Download", "Open online"])
                .default(0)
                .interact_on_opt(&Term::stderr())?
            {
                Some(0) => {
                    let url = paper.download_url();
                    download_and_save(paper.metadata().clone(), url, lib, output)?;
                }
                Some(1) => {
                    open::that(paper.ee.raw())?;
                }
                _ => {
                    bail!("User did not select any remote! Aborting!");
                }
            }
        }
    }
    Ok(())
}

pub fn download_and_save(
    metadata: PaperInfo,
    download_url: PaperUrl,
    lib: &mut Library,
    output: Option<&PathBuf>,
) -> Result<()> {
    let dest = if let Some(output) = output {
        if output.is_file() {
            output.to_owned().with_extension("pdf")
        } else {
            output.join(metadata.default_filename())
        }
    } else {
        config::xivar_document_dir()?.join(metadata.default_filename())
    }
    .with_extension("pdf");

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::default_spinner().template("{msg} {spinner:.cyan/blue} "),
    );
    spinner.set_message("Downloading");
    spinner.enable_steady_tick(10);
    task::block_on(download_pdf(&download_url.raw(), &dest))?;
    spinner.abandon_with_message(
        &style(format!("Saved file to {:?}!", dest))
            .green()
            .bold()
            .to_string(),
    );
    let paper = LocalPaper {
        metadata,
        location: dest.clone(),
        ees: vec![download_url],
    };
    lib.add(&dest, paper);
    open::that(dest)?;
    lib.save()
}

pub fn open_local_otherwise_download(
    paper: LocalPaper,
    lib: &mut Library,
    output: Option<&PathBuf>,
) -> Result<()> {
    if paper.exists() {
        open::that(paper.location)?;
    } else {
        match Select::with_theme(&ColorfulTheme::default())
            .items(&paper.ees)
            .with_prompt("Paper is not located at old location! Do you want to")
            .default(0)
            .interact_on_opt(&Term::stderr())?
        {
            Some(i) => {
                download_and_save(paper.metadata, paper.ees[i].clone(), lib, output)?;
            }
            _ => {
                bail!("User did not select any remote! Aborting!");
            }
        }
    }
    Ok(())
}

pub fn search_and_select(lib: &Library, search_string: &str) -> Result<Paper> {
    let terms = vec![search_string.to_owned()];
    let query = Query::builder().terms(terms).build();
    let fzf = Fzf::new()?;
    let handle = fzf.fetch_and_write(remotes::fetch_all_and_merge(lib, query));
    task::block_on(handle)?;
    fzf.wait_for_selection()
}

pub async fn download_pdf(url: &str, out_path: &PathBuf) -> Result<()> {
    let mut response = surf::get(&url).await.map_err(|err| anyhow!(err))?;
    let body = response.body_bytes().await.map_err(|err| anyhow!(err))?;
    let mut file = std::fs::File::create(out_path.with_extension("pdf"))?;
    file.write_all(&body)?;
    Ok(())
}
