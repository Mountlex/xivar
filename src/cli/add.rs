use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Clap;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use lopdf::{Document, Object};

use super::{util, Command};

use crate::{
    config,
    store::{get_store_results, Library},
    Identifier, Paper, PaperUrl, Query,
};

#[derive(Clap, Debug)]
pub struct Add {
    #[clap(parse(from_os_str))]
    pdf_file: PathBuf,
}

impl Command for Add {
    fn run(&self) -> Result<()> {
        if self.pdf_file.is_file() && "pdf" == self.pdf_file.extension().unwrap() {
            let data_dir = config::xivar_data_dir()?;
            let mut lib = Library::open(&data_dir)?;
            if lib.find_paper_by_path(&self.pdf_file).is_some() && !Confirm::new().with_prompt("This paper is already in your library! You can add another entry to your library by using another ID. Continue?").default(false).interact()? {
                return Ok(());
            }

            let spinner = indicatif::ProgressBar::new_spinner();
            spinner.set_style(
                indicatif::ProgressStyle::default_spinner().template("{msg} {spinner:.cyan/blue} "),
            );
            spinner.set_message("Reading PDF");
            spinner.enable_steady_tick(10);

            let doc = Document::load(&self.pdf_file)?;
            let authors = get_author(&doc);
            let title = get_title(&doc);

            spinner.finish_and_clear();

            if let Some(ref title) = title {
                if !get_store_results(Query::builder().terms(vec![title.to_owned()]).build(), &lib)
                    .is_empty()
                {
                    println!(
                        "Note that there already is a paper with the title {} in your library!",
                        style(title).bold().cyan()
                    );
                }
            }

            let mut options = vec![
                "Enter metadata manually".to_owned(),
                "Search online".to_owned(),
            ];
            if let Some(ref title) = title {
                options.push(format!("Search title \"{}\" online", style(title).bold()));
            }

            loop {
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .items(&options)
                    .default(0)
                    .interact_on_opt(&Term::stderr())?;
                let paper = match selection {
                    Some(0) => enter_manually(title.as_deref(), authors.as_deref()),
                    Some(1) => {
                        let search_string: String =
                            Input::new().with_prompt("Query").interact_text()?;
                        util::search_and_select(&search_string)
                    }
                    Some(2) => util::search_and_select(title.as_deref().unwrap()),
                    _ => {
                        bail!("Aborting!")
                    }
                };
                if let Ok(paper) = paper {
                    lib.add(&self.pdf_file, paper);
                    println!("{}", style("Added paper to library!").green().bold());
                    break;
                }
            }
        } else {
            println!("{}", style("The given file is no PDF!").red().bold());
        }

        Ok(())
    }
}

fn enter_manually(title: Option<&str>, authors: Option<&str>) -> Result<Paper> {
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
    Ok(Paper {
        id: Identifier::Custom(id),
        title,
        authors: authors.split(",").map(|a| a.trim().to_owned()).collect(),
        year,
        url: PaperUrl::new(url),
        local_path: None,
    })
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
