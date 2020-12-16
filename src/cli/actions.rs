use anyhow::Result;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};
use std::{fmt::Display, path::PathBuf};

use crate::{
    remotes::{Paper, PaperHit, RemoteTag},
    PaperUrl,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Download(PaperUrl, PaperHit),
    OpenLocal(PathBuf),
    OpenRemote(PaperUrl, PaperHit),
    EnterUrl(PaperHit),
    CopyLocal(Vec<PaperUrl>, PaperHit),
    ProcessHit(PaperHit),
    SelectHit(Paper),
    Finish,
    Back,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Download(_, _) => write!(f, "Download"),
            Action::OpenLocal(_) => write!(f, "Local"),
            Action::EnterUrl(_) => write!(f, "Enter PDF-Url"),
            Action::CopyLocal(_, _) => write!(f, "Enter Local PDF"),
            Action::OpenRemote(url, _) => write!(f, "{}", url),
            Action::ProcessHit(hit) => write!(f, "{}", hit.remote_tag()),
            Action::SelectHit(_) => write!(f, ""),
            Action::Finish => write!(f, "Finish"),
            Action::Back => write!(f, "Back"),
        }
    }
}

pub fn select_action(prompt: String, mut actions: Vec<Action>) -> Result<Action> {
    match Select::with_theme(&ColorfulTheme::default())
        .items(&actions)
        .default(0)
        .with_prompt(prompt)
        .interact_on_opt(&Term::stderr())?
    {
        Some(i) => Ok(actions.remove(i)),
        None => Ok(Action::Back),
    }
}
