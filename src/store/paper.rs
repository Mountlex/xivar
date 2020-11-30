use super::query::Query;

use serde::{Deserialize, Serialize};
use std::path::Path;


#[derive(Debug, Default, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct ArxivIdentifier {
    year: u8,
    month: u8,
    number: u64,
}

impl ArxivIdentifier {
    fn parse_string(id: String) -> Self {
        Self::default()
    }

    pub fn as_str(&self) -> String {
        format!("{:2}{:2}.{:5}", self.year, self.month, self.number)
    }
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
    location: Option<String>,
}

impl From<arxiv::Arxiv> for Paper {
    fn from(p: arxiv::Arxiv) -> Self {
        Paper {
            id: ArxivIdentifier::parse_string(p.id),
            title: p.title,
            authors: p.authors,
            published: p.published,
            updated: p.updated,
            pdf_url: p.pdf_url,
            summary: p.summary,
            location: None
        }
    }
}

impl PartialEq for Paper {
    fn eq(&self, other: &Paper) -> bool {
        self.id == other.id
    }
}

impl Eq for Paper {}

impl Paper {
    pub fn exists(&self) -> bool {
        if let Some(ref loc) = self.location {
            Path::new(&loc).exists()
        } else {
            false
        }
    }

    pub fn matches(&self, query: &Query) -> bool {
        match query {
            Query::Full(qstring) => {
                any_match(&qstring, &self.authors.join(" "))
                    | any_match(&qstring, &self.title)
            }
            Query::Author(qstring) => any_match(&qstring, &self.authors.join(" ")),
            Query::Title(qstring) => any_match(&qstring, &self.title),
        }
    }
}

impl std::fmt::Display for Paper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} [{}]", self.title, self.authors.join(", "))
    }
}

fn any_match(qstring: &str, sstring: &str) -> bool {
    qstring.split_whitespace().any(|s| sstring.contains(s))
}
