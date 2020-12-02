use super::identifier::Identifier;
use super::query::Query;

use serde::{Deserialize, Serialize};
use std::path::Path;

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
}

impl PartialEq for Paper {
    fn eq(&self, other: &Paper) -> bool {
        self.id == other.id
    }
}

impl Eq for Paper {}

impl Paper {
    pub fn matches(&self, query: Query) -> bool {
        match query {
            Query::Full(qstrings) => {
                any_match(qstrings, &self.authors.join(" ")) | any_match(qstrings, &self.title)
            }
            Query::Author(qstrings) => any_match(qstrings, &self.authors.join(" ")),
            Query::Title(qstrings) => any_match(qstrings, &self.title),
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
        write!(
            f,
            "{} [{} by {}] {}",
            self.title,
            self.year,
            self.authors.join(", "),
            self.url.preprint().unwrap_or("".to_owned())
        )
    }
}

fn any_match(qstrings: &[String], sstring: &str) -> bool {
    qstrings
        .iter()
        .any(|s| sstring.to_lowercase().contains(&s.to_lowercase()))
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaperUrl(String);

impl PaperUrl {
    pub fn new(url: String) -> Self {
        PaperUrl(url)
    }

    pub fn preprint(&self) -> Option<String> {
        if self.0.contains("arxiv") {
            Some("arXiv".to_owned())
        } else {
            None
        }
    }

    pub fn raw(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaperCopy {
    pub paper: Paper,
    pub location: std::path::PathBuf,
}

impl PaperCopy {
    pub fn exists(&self) -> bool {
        Path::new(&self.location).exists()
    }
}

impl MatchByTitle for PaperCopy {
    fn matches_title(&self, title: &str) -> bool {
        self.paper.matches_title(title)
    }
}

impl std::fmt::Display for PaperCopy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} on disk at {:?}", self.paper, self.location)
    }
}
