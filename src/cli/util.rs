use std::path::PathBuf;

use anyhow::{anyhow, Result};
use async_std::task;

use std::io::Write;

use console::style;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};

use crate::{config, fzf::Fzf, remotes, Query};

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
                spinner.abandon_with_message(
                    &style(format!("Saved file to {:?}!", dest))
                        .green()
                        .bold()
                        .to_string(),
                );

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
    paper: Paper,
    lib: &mut Library,
    output: &Option<PathBuf>,
) -> Result<()> {
    if paper.exists() {
        open::that(paper.local_path.unwrap())?;
    } else {
        println!("Paper is not located at old location! Do you want to");
        select_remote_or_download(paper, lib, output)?;
    }
    Ok(())
}

pub fn search_and_select(search_string: &str) -> Result<Paper> {
    let terms = vec![search_string.to_owned()];
    let query = Query::builder().terms(terms).build();
    let fzf = Fzf::new()?;
    let online_handle = fzf.fetch_and_write(remotes::fetch_all_and_merge(query));
    task::block_on(online_handle)?;
    fzf.wait_for_selection()
}

pub async fn download_pdf(url: &str, out_path: &PathBuf) -> Result<()> {
    let mut response = surf::get(&url).await.map_err(|err| anyhow!(err))?;
    let body = response.body_bytes().await.map_err(|err| anyhow!(err))?;
    let mut file = std::fs::File::create(out_path.with_extension("pdf"))?;
    file.write_all(&body)?;
    Ok(())
}
