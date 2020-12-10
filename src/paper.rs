use super::identifier::Identifier;
use super::query::Query;

use console::style;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub trait MatchByTitle {
    fn matches_title(&self, title: &str) -> bool;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Paper {
    pub id: Identifier,
    pub title: String,
    pub authors: Vec<String>,
    pub year: String,
    pub url: PaperUrl,
    pub local_path: Option<PathBuf>,
}

impl PartialEq for Paper {
    fn eq(&self, other: &Paper) -> bool {
        self.id == other.id
    }
}

impl Eq for Paper {}

impl Paper {
    pub fn matches(&self, query: &Query) -> bool {
        if let Some(ref terms) = query.terms {
            any_match(terms.as_slice(), &self.authors.join(" "))
                | any_match(terms.as_slice(), &self.title)
        } else {
            true
        }
    }

    pub fn default_filename(&self) -> String {
        format!("{}", self.id).replace(".", "-")
    }

    pub fn preprint(&self) -> Option<Preprint> {
        if let Identifier::Arxiv(ref id) = self.id {
            Some(Preprint::Arxiv(PaperUrl::new(format!(
                "https://arxiv.org/pdf/{}.pdf",
                id
            ))))
        } else {
            None
        }
    }

    pub fn exists(&self) -> bool {
        if let Some(ref path) = self.local_path {
            Path::new(path).exists()
        } else {
            false
        }
    }
}

impl MatchByTitle for Paper {
    fn matches_title(&self, title: &str) -> bool {
        self.title.trim() == title
    }
}

impl std::fmt::Display for Paper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let preprint_server = self
            .preprint()
            .map(|p| p.server_name())
            .unwrap_or("".to_owned());
        let local = self
            .local_path
            .as_ref()
            .map(|_| "local")
            .unwrap_or_default();
        write!(
            f,
            "{} [{} by {}] {} {}",
            style(self.title.clone()).bold(),
            style(self.year.clone()).yellow(),
            self.authors.join(", "),
            style(preprint_server).bold().cyan(),
            style(local).blue().bold()
        )
    }
}

fn any_match(qstrings: &[String], sstring: &str) -> bool {
    if qstrings.is_empty() {
        true
    } else {
        qstrings
            .iter()
            .any(|s| sstring.to_lowercase().contains(&s.to_lowercase()))
    }
}

pub enum Preprint {
    Arxiv(PaperUrl),
}

impl Preprint {
    pub fn server_name(&self) -> String {
        match self {
            Preprint::Arxiv(_) => "arXiv".to_owned(),
        }
    }

    pub fn pdf_url(&self) -> &PaperUrl {
        match self {
            Preprint::Arxiv(url) => url,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaperUrl(String);

impl PaperUrl {
    pub fn new(url: String) -> Self {
        PaperUrl(url)
    }

    pub fn raw(&self) -> String {
        self.0.clone()
    }
}
