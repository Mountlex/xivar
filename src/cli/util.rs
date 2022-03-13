use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use indicatif::{ProgressBar, ProgressStyle};
use remotes::{
    local::{Library, LocalPaper},
    Paper, PaperHit, RemoteTag,
};
use tokio::io::AsyncWriteExt;

use console::style;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};

use crate::{config, finder, remotes, PaperInfo, PaperUrl, Query};

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
            Some(i) => Ok(hits[i].clone()),
            _ => bail!("User did not select any remote! Aborting!"),
        }
    }
}

pub async fn async_download_and_save(
    metadata: PaperInfo,
    download_url: PaperUrl,
    output: Option<&PathBuf>,
) -> Result<LocalPaper> {
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

    download_pdf(&download_url.raw(), &dest).await?;

    open::that(&dest)?;
    Ok(LocalPaper {
        metadata,
        location: dest,
        ees: vec![download_url],
    })
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
    //tokio::task::spawn(download_pdf(&download_url.raw(), &dest));
    spinner.abandon_with_message(
        style(format!("Saved file to {:?}!", dest))
            .green()
            .bold()
            .to_string(),
    );
    open::that(&dest)?;
    let paper = LocalPaper {
        metadata,
        location: dest,
        ees: vec![download_url],
    };
    lib.add(paper);
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

pub fn search_and_select(
    lib: &Library,
    terms: Vec<String>,
    max_hits: Option<u32>,
) -> Result<Paper> {
    let spinner = ProgressBar::new_spinner();
    spinner
        .set_style(ProgressStyle::default_spinner().template("{msg:.bold} {spinner:.cyan/blue}"));
    spinner.set_message("Searching");
    spinner.enable_steady_tick(10);

    let query = Query::builder().terms(terms).max_hits(max_hits).build();

    let papers = vec![]; //= tokio::task::spawn(remotes::fetch_all_and_merge(&lib, query);
    spinner.finish_and_clear();

    finder::show_and_select(papers.into_iter())
}

pub async fn download_pdf(url: &str, out_path: &PathBuf) -> Result<()> {
    let response = reqwest::get(&*url).await.map_err(|err| anyhow!(err))?;
    let body = response.bytes().await.map_err(|err| anyhow!(err))?;
    let mut file = tokio::fs::File::create(out_path.with_extension("pdf")).await?;
    file.write_all(&body).await?;
    Ok(())
}
