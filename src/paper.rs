use super::identifier::Identifier;
use super::query::Query;

use console::style;
use serde::{Deserialize, Serialize};

pub trait MatchByTitle {
    fn matches_title(&self, title: &str) -> bool;
}

pub trait PaperRef: std::fmt::Display {
    fn metadata(&self) -> &PaperInfo;
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct PaperInfo {
    pub id: Option<Identifier>,
    pub title: PaperTitle,
    pub venue: String,
    pub authors: Vec<String>,
    pub year: String,
}

impl PaperInfo {
    pub fn matches(&self, query: &Query) -> bool {
        if let Some(ref terms) = query.terms {
            any_match(terms.as_slice(), &self.authors.join(" "))
                | any_match(terms.as_slice(), &self.title.normalized())
        } else {
            true
        }
    }

    pub fn default_filename(&self) -> String {
        format!("{}", self.id.as_ref().unwrap()).replace(".", "-")
    }
}

impl std::fmt::Display for PaperInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}. [{}]",
            style(format!("{}", self.title)).bold(),
            self.authors.join(", "),
        )
    }
}

impl PartialEq for PaperInfo {
    fn eq(&self, other: &PaperInfo) -> bool {
        match (self.id.as_ref(), other.id.as_ref()) {
            (Some(a), Some(b)) => a == b,
            _ => self.title == other.title && self.venue == other.venue && self.year == other.year,
        }
    }
}

impl Eq for PaperInfo {}

fn any_match(qstrings: &[String], sstring: &str) -> bool {
    if qstrings.is_empty() {
        true
    } else {
        qstrings
            .iter()
            .any(|s| sstring.to_lowercase().contains(&s.to_lowercase()))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct PaperTitle {
    words: Vec<String>,
}

impl PaperTitle {
    pub fn new(title: String) -> Self {
        let words = title
            .replace(".", "")
            .replace("$", "")
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        PaperTitle { words }
    }

    pub fn normalized(&self) -> String {
        self.words
            .iter()
            .map(|s| s.to_lowercase())
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl std::fmt::Display for PaperTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.words.join(" "))
    }
}

impl PartialEq for PaperTitle {
    fn eq(&self, other: &PaperTitle) -> bool {
        self.normalized() == other.normalized()
    }
}

impl Eq for PaperTitle {}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct PaperUrl(String);

impl PaperUrl {
    pub fn new(url: String) -> Self {
        PaperUrl(url)
    }

    pub fn raw(&self) -> String {
        self.0.clone()
    }
}

impl std::fmt::Display for PaperUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
