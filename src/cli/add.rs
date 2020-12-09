use std::path::PathBuf;

use anyhow::Result;
use async_std::task;
use clap::Clap;
use console::style;
use dialoguer::{Confirm, Input};
use lopdf::{Document, Object};

use super::Command;

use crate::{config, fzf, remotes::dblp, store::Library, Identifier, Paper, PaperUrl, Query};

#[derive(Clap, Debug)]
pub struct Add {
    #[clap(parse(from_os_str))]
    pdf_file: PathBuf,

    #[clap(long)]
    offline: bool,
}

impl Command for Add {
    fn run(&self) -> Result<()> {
        if "pdf" == self.pdf_file.extension().unwrap() {
            let data_dir = config::xivar_data_dir()?;
            let mut lib = Library::open(&data_dir)?;
            let doc = Document::load(&self.pdf_file)?;

            let authors = get_author(&doc);
            let title = get_title(&doc);
            if let Some(ref title) = title {
                if !self.offline
                    && Confirm::new()
                        .with_prompt(format!("Search title \"{}\" online?", style(title).bold()))
                        .default(true)
                        .interact()?
                {
                    let terms = vec![title.to_owned()];
                    let query = Query::builder().terms(&terms).build();
                    let fzf = fzf::Fzf::new()?;
                    let online_handle = fzf.fetch_and_write(dblp::fetch_query(&query));
                    if task::block_on(online_handle).is_ok() {
                        if let Ok(paper) = fzf.wait_for_selection() {
                            lib.add(&self.pdf_file, paper);
                            println!("{}", style("Added paper to library!").green().bold());
                            return Ok(());
                        }
                    }
                }
            }

            let title: String = Input::new()
                .with_prompt("Title")
                .with_initial_text(title.unwrap_or_default())
                .interact_text()?;
            let authors: String = Input::new()
                .with_prompt("Authors")
                .with_initial_text(authors.unwrap_or_default())
                .interact_text()?;
            let year: String = Input::new().with_prompt("Year").interact_text()?;
            let url: String = Input::new().with_prompt("Url").interact_text()?;
            let id: String = Input::new().with_prompt("Identifier").interact_text()?;
            let paper = Paper {
                id: Identifier::Custom(id),
                title,
                authors: authors.split(",").map(|a| a.trim().to_owned()).collect(),
                year,
                url: PaperUrl::new(url),
                local_path: None,
            };
            lib.add(&self.pdf_file, paper);
            println!("{}", style("Added paper to library!").green().bold());
        } else {
            println!("{}", style("The given file is no PDF!").red().bold());
        }

        Ok(())
    }
}

fn get_title(doc: &Document) -> Option<String> {
    get_info_field(doc, "Title")
}

fn get_author(doc: &Document) -> Option<String> {
    get_info_field(doc, "Author")
}

fn get_info_field(doc: &Document, field_name: &str) -> Option<String> {
    doc.trailer.get(b"Info").ok().and_then(|info| {
        match *info {
            Object::Dictionary(ref dict) => Some(dict),
            Object::Reference(ref id) => doc.objects.get(id).and_then(|o| o.as_dict().ok()),
            _ => None,
        }
        .and_then(|dict| {
            dict.get(field_name.as_bytes()).ok().and_then(|obj| {
                let field = std::str::from_utf8(obj.as_str().unwrap())
                    .unwrap()
                    .replace("(", "")
                    .replace(")", "");
                if field.is_empty() {
                    None
                } else {
                    Some(field)
                }
            })
        })
    })
}
