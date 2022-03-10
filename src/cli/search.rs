use std::path::{Path, PathBuf};

use super::{
    actions::{self, Action},
    util, Command,
};
use crate::remotes;
use crate::{config, PaperUrl};

use actions::select_action;
use anyhow::Result;
use clap::Parser;
use console::style;
use dialoguer::Input;
use remotes::{
    local::{Library, LocalPaper},
    PaperHit,
};

#[derive(Parser, Debug)]
#[clap(about = "Search remotes and your local library")]
pub struct Search {
    search_terms: Vec<String>,

    /// Specify an unique download location
    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    /// Caps the number of hits from a single remote
    #[clap(short, long)]
    num_hits: Option<u32>,
}

impl Command for Search {
    fn run(&self) -> Result<()> {
        let data_dir = config::xivar_data_dir()?;
        let mut lib = Library::open(&data_dir)?;

        let paper = util::search_and_select(&lib, self.search_terms.clone(), self.num_hits)?;

        let mut stack: Vec<Action> = vec![Action::SelectHit(paper)];
        while let Some(current) = stack.last().cloned() {
            match current.execute(&mut lib)? {
                Action::Finish => break,
                Action::Back => {
                    stack.pop(); // Pop back action
                    stack.pop(); // Pop previous "real" action
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
                let actions = vec![
                    Action::EnterUrl(hit.clone()),
                    Action::CopyLocal(vec![url], hit.clone()),
                    Action::Finish,
                ];
                select_action("Select action".to_owned(), actions)
            }
            Action::EnterUrl(hit) => {
                if let Ok(pdf_url) = Input::new().with_prompt("PDF-Url").interact_text() {
                    Ok(Action::Download(PaperUrl::new(pdf_url), hit))
                } else {
                    Ok(Action::Finish)
                }
            }
            Action::CopyLocal(ees, hit) => {
                if let Ok(location) = Input::<String>::new()
                    .with_prompt("Local path")
                    .interact_text()
                {
                    let path = Path::new(&location);
                    let dest =
                        config::xivar_document_dir()?.join(hit.metadata().default_filename());
                    std::fs::copy(path, &dest)?;
                    println!(
                        "{}",
                        style(format!("Saved pdf to {:?}!", dest)).bold().green()
                    );
                    let paper = LocalPaper {
                        metadata: hit.metadata().to_owned(),
                        location: dest,
                        ees,
                    };
                    lib.add(paper);
                    lib.save()?;
                }
                Ok(Action::Finish)
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
