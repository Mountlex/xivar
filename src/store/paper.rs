use super::query::Query;

use serde::{Deserialize, Serialize};
use std::path::Path;


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ArxivIdentifier {
    year: u8,
    month: u8,
    number: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paper {
    id: ArxivIdentifier,
    title: String,
    authors: String,
    keywords: String,
    location: String,
}

impl PartialEq for Paper {
    fn eq(&self, other: &Paper) -> bool {
        self.id == other.id
    }
}

impl Eq for Paper {}

impl Paper {
    pub fn exists(&self) -> bool {
        Path::new(&self.location).exists()
    }

    pub fn matches(&self, query: &Query) -> bool {
        match query {
            Query::Full(qstring) => {
                any_match(&qstring, &self.authors)
                    | any_match(&qstring, &self.title)
                    | any_match(&qstring, &self.keywords)
            }
            Query::Author(qstring) => any_match(&qstring, &self.authors),
            Query::Title(qstring) => any_match(&qstring, &self.title),
        }
    }
}

fn any_match(qstring: &str, sstring: &str) -> bool {
    qstring.split_whitespace().any(|s| sstring.contains(s))
}
