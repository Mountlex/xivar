use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
};

use crate::{
    library::LocalPaper,
    query::QueryTerm,
    remotes::{arxiv::ArxivPaper, dblp::DBLPPaper},
};

use super::identifier::Identifier;
use super::query::Query;
use anyhow::Result;
use console::style;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub trait MatchByTitle {
    fn matches_title(&self, title: &str) -> bool;
}

pub trait PaperRef: std::fmt::Display {
    fn metadata(&self) -> &PaperInfo;
}

pub fn merge_papers<I: Iterator<Item = PaperHit>>(hits: I) -> Result<Vec<Paper>> {
    let mut papers: Vec<Paper> = hits
        .map(|p| (p.metadata().title.normalized(), p))
        .into_group_map()
        .into_iter()
        .map(|(_, v)| Paper::new(v))
        .collect();

    papers.sort_by_key(|r| r.metadata().year.to_owned());
    papers.reverse();
    Ok(papers)
}

pub fn merge_to_papers<I: Iterator<Item = PaperHit>>(papers: &mut Vec<Paper>, hits: I) {
    *papers = papers
        .iter_mut()
        .flat_map(|p| p.0.clone())
        .chain(hits)
        .map(|p| (p.metadata().title.normalized(), p))
        .into_group_map()
        .into_iter()
        .map(|(_, mut v)| {
            v.sort_by(|a, b| match (a, b) {
                (PaperHit::Local(_), PaperHit::Local(_)) => Ordering::Equal,
                (PaperHit::Arxiv(_), PaperHit::Arxiv(_)) => Ordering::Equal,
                (PaperHit::Dblp(_), PaperHit::Dblp(_)) => Ordering::Equal,
                (PaperHit::Local(_), _) => Ordering::Less,
                (PaperHit::Arxiv(_), PaperHit::Dblp(_)) => Ordering::Less,
                (PaperHit::Arxiv(_), PaperHit::Local(_)) => Ordering::Greater,
                (PaperHit::Dblp(_), _) => Ordering::Greater,
            });
            Paper::new(v)
        })
        .collect();
    papers.sort_by_key(|r| r.metadata().year.to_owned());
    papers.reverse();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaperHit {
    Local(LocalPaper),
    Arxiv(ArxivPaper),
    Dblp(DBLPPaper),
}

impl PaperHit {
    pub fn metadata(&self) -> &PaperInfo {
        match self {
            PaperHit::Arxiv(paper) => paper.metadata(),
            PaperHit::Dblp(paper) => paper.metadata(),
            PaperHit::Local(paper) => paper.metadata(),
        }
    }

    pub fn remote_tag(&self) -> String {
        match self {
            PaperHit::Arxiv(paper) => paper.remote_tag(),
            PaperHit::Dblp(paper) => paper.remote_tag(),
            PaperHit::Local(paper) => paper.remote_tag(),
        }
    }
}

impl Display for PaperHit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PaperHit::Arxiv(paper) => write!(f, "{}", paper),
            PaperHit::Dblp(paper) => write!(f, "{}", paper),
            PaperHit::Local(paper) => write!(f, "{}", paper),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Paper(pub Vec<PaperHit>);

impl Paper {
    pub fn new(hits: Vec<PaperHit>) -> Self {
        Paper(hits)
    }

    pub fn hits(&self) -> &[PaperHit] {
        &self.0
    }

    pub fn metadata(&self) -> &PaperInfo {
        self.0.first().unwrap().metadata()
    }
}

impl PartialEq for Paper {
    fn eq(&self, other: &Paper) -> bool {
        self.0.first().unwrap().metadata() == other.0.first().unwrap().metadata()
    }
}

impl Eq for Paper {}

impl Display for Paper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ", self.metadata())?;
        for hit in self.0.iter() {
            write!(f, "{} ", hit.remote_tag())?;
        }
        write!(f, "")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaperInfo {
    pub id: Option<Identifier>,
    pub title: PaperTitle,
    pub venue: Venue,
    pub authors: Vec<String>,
    pub year: String,
    pub summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Venue {
    Journal(String),
    Conf(String),
    Arxiv(String),
}

impl Display for Venue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Venue::Journal(name) => write!(f, "{}", name),
            Venue::Conf(name) => write!(f, "{}", name),
            Venue::Arxiv(name) => write!(f, "{}", name),
        }
    }
}

impl PaperInfo {
    pub fn matches(&self, query: &Query) -> bool {
        let single_string = self.single_string();
        let terms: Vec<String> = single_string
            .split_whitespace()
            .map(|t| t.trim().to_lowercase())
            .collect();

        query.into_iter().all(|term| match term {
            QueryTerm::Exact(q) => terms.iter().any(|t| q == t),
            QueryTerm::Prefix(q) => terms.iter().any(|t| t.starts_with(q)),
        })
    }

    fn single_string(&self) -> String {
        format!(
            "{} {} {} {}",
            self.title.normalized(),
            self.authors.join(" "),
            self.venue,
            self.year
        )
        .to_lowercase()
    }

    pub fn default_filename(&self) -> String {
        let first_author = self
            .authors
            .first()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_lowercase();
        let other_names = self
            .authors
            .iter()
            .skip(1)
            .flat_map(|a| a.chars().next().map(|c| c.to_ascii_uppercase()))
            .join("");
        let title = self
            .title
            .words
            .clone()
            .into_iter()
            .take(2)
            .map(|w| w.to_uppercase())
            .collect::<Vec<String>>()
            .join("")
            .to_lowercase();
        format!(
            "{}{}{}{}",
            first_author,
            other_names,
            self.year[2..].to_owned(),
            title
        )
        .replace('/', "-")
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaperTitle {
    pub words: Vec<String>,
}

impl PaperTitle {
    pub fn new(title: String) -> Self {
        let words = title
            .replace('.', "")
            .replace('$', "")
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
