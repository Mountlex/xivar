use super::query::Query;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;
use regex::Regex;

#[derive(Debug, Default, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct ArxivIdentifier {
    year: u32,
    month: u32,
    number: String,
}

impl ArxivIdentifier {
    fn parse_string(id: String) -> Result<Self> {
        let temp = id.split("/").last().unwrap();
        let re = Regex::new(r"(\d{2})(\d{2})\.?(.+)").unwrap();
        if let Some(capture) = re.captures(temp) {
            Ok(ArxivIdentifier {
                year: capture[1].parse::<u32>().unwrap(),
                month: capture[2].parse::<u32>().unwrap(),
                number: capture[3].to_owned(),
            })
        } else {
            bail!("Cannot read identifier {}!", id)
        }
    }

    pub fn as_str(&self) -> String {
        format!("{:0>2}{:0>2}-{}", self.year, self.month, self.number)
    }
}

pub trait MatchByTitle {
    fn matches_title(&self, title: &str) -> bool;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Paper {
    pub id: ArxivIdentifier,
    pub title: String,
    updated: String,
    published: String,
    summary: String,
    pub pdf_url: String,
    authors: Vec<String>,
}



impl PartialEq for Paper {
    fn eq(&self, other: &Paper) -> bool {
        self.id == other.id
    }
}

impl Eq for Paper {}

impl Paper {
    pub fn from_arxiv(p: arxiv::Arxiv) -> Result<Self> {
        Ok(Paper {
            id: ArxivIdentifier::parse_string(p.id)?,
            title: p.title.split_whitespace().collect::<Vec<&str>>().join(" "),
            authors: p.authors,
            published: p.published,
            updated: p.updated,
            pdf_url: p.pdf_url,
            summary: p.summary,
        })
    }

    

    pub fn matches(&self, query: Query) -> bool {
        match query {
            Query::Full(qstrings) => {
                any_match(qstrings, &self.authors.join(" "))
                    | any_match(qstrings, &self.title)
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
        write!(f, "{} [{}]", self.title, self.authors.join(", "))
    }
}

fn any_match(qstrings: &[String], sstring: &str) -> bool {
    qstrings.iter().any(|s| sstring.to_lowercase().contains(&s.to_lowercase()))
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