use std::path::PathBuf;

use super::{
    actions::{self, Action},
    util, Command,
};
use crate::fzf;
use crate::remotes;
use crate::{config, PaperUrl, Query};

use actions::select_action;
use anyhow::Result;
use async_std::task;
use clap::Clap;
use dialoguer::{Confirm, Input};
use fzf::Fzf;
use indicatif::{ProgressBar, ProgressStyle};
use remotes::{local::Library, Paper, PaperHit};

#[derive(Clap, Debug)]
#[clap(about = "Search remotes and your local library")]
pub struct Search {
    search_terms: Vec<String>,

    #[clap(
        short,
        long,
        about = "Specify an unique download location",
        parse(from_os_str)
    )]
    output: Option<PathBuf>,

    #[clap(
        short,
        long,
        about = "Caps the number of hits from a single remote",
        default_value = "50"
    )]
    num_hits: u32,
}

impl Command for Search {
    fn run(&self) -> Result<()> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner().template("{msg:.bold} {spinner:.cyan/blue}"),
        );
        spinner.set_message("Searching");
        spinner.enable_steady_tick(10);

        let query = Query::builder()
            .terms(self.search_terms.clone())
            .max_hits(self.num_hits)
            .build();
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let papers = task::block_on(remotes::fetch_all_and_merge(&lib, query))?;
        spinner.finish_and_clear();

        let mut fzf: Fzf<Paper> = Fzf::new()?;
        fzf.write_all(papers);
        let paper = fzf.wait_for_selection()?;
        let mut stack: Vec<Action> = vec![Action::SelectHit(paper)];
        while let Some(current) = stack.last().cloned() {
            match current.execute(&mut lib)? {
                Action::Finish => break,
                Action::Back => {
                    stack.pop();
                }
                a => stack.push(a),
            }
        }
        lib.save()
    }
}

trait SearchAction {
    fn execute(self, lib: &mut Library) -> Result<Action>;
}

impl SearchAction for Action {
    fn execute(self, lib: &mut Library) -> Result<Action> {
        match self {
            Action::Download(url, hit) => {
                util::download_and_save(hit.metadata().clone(), url, lib, None)?;
                Ok(Action::Finish)
            }
            Action::OpenLocal(url) => {
                open::that(url)?;
                Ok(Action::Finish)
            }
            Action::OpenRemote(url, hit) => {
                open::that(url.raw())?;
                if Confirm::new()
                    .with_prompt("Do you want to download the paper via a pdf-url?")
                    .default(true)
                    .interact()?
                {
                    if let Ok(pdf_url) = Input::new().with_prompt("PDF-Url").interact_text() {
                        Ok(Action::Download(PaperUrl::new(pdf_url), hit))
                    } else {
                        Ok(Action::Finish)
                    }
                } else {
                    Ok(Action::Finish)
                }
            }
            Action::SelectHit(paper) => {
                let mut hits = paper.0;
                if hits.is_empty() {
                    Ok(Action::Finish)
                } else if hits.len() == 1 {
                    Ok(Action::ProcessHit(hits.remove(0)))
                } else {
                    let actions = hits
                        .into_iter()
                        .map(|hit| Action::ProcessHit(hit))
                        .collect();
                    select_action("Select hit".to_owned(), actions)
                }
            }
            Action::ProcessHit(hit) => match hit {
                PaperHit::Local(paper) => Ok(Action::OpenLocal(paper.location)),
                PaperHit::Dblp(ref paper) => {
                    let urls = vec![paper.ee.raw(), paper.url.raw()];
                    let actions = urls
                        .into_iter()
                        .map(|url| Action::OpenRemote(PaperUrl::new(url), hit.clone()))
                        .collect();
                    select_action("Select reference".to_owned(), actions)
                }
                PaperHit::Arxiv(ref paper) => {
                    let actions = vec![
                        Action::Download(paper.download_url(), hit.clone()),
                        Action::OpenRemote(paper.ee.clone(), hit.clone()),
                    ];
                    select_action("Select action".to_owned(), actions)
                }
            },
            Action::Finish => panic!("Do not call execute on finish action!"),
            Action::Back => panic!("Do not call execute on back action!"),
        }
    }
}
