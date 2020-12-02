use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub enum Identifier {
    Arxiv(ArxivIdentifier),
    Doi(Doi),
    Custom(String),
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Identifier::Arxiv(arxiv) => write!(f, "{}", arxiv),
            Identifier::Doi(doi) => write!(f, "{}", doi),
            Identifier::Custom(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct ArxivIdentifier {
    year: u32,
    month: u32,
    number: String,
}

impl ArxivIdentifier {
    pub fn parse_string(id: &str) -> Result<Self> {
        let temp = id.split("/").last().unwrap();
        let re = Regex::new(r"(\d{2})(\d{2})\.?(.+)").unwrap();
        if let Some(capture) = re.captures(temp) {
            Ok(ArxivIdentifier {
                year: capture[1].parse::<u32>().unwrap(),
                month: capture[2].parse::<u32>().unwrap(),
                number: capture[3].to_owned(),
            })
        } else {
            bail!("Cannot read arxive-url {}!", id)
        }
    }
}

impl std::fmt::Display for ArxivIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:0>2}{:0>2}.{}", self.year, self.month, self.number)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Doi {
    organization: u32,
    id: String,
}

impl Doi {
    pub fn parse_doi(doi_string: &str) -> Result<Self> {
        let re = Regex::new(r"10.(\d+)/(.+)").unwrap();
        if let Some(capture) = re.captures(doi_string) {
            Ok(Doi {
                organization: capture[1].parse::<u32>().unwrap(),
                id: capture[2].to_owned(),
            })
        } else {
            bail!("Cannot read doi {}!", doi_string)
        }
    }
}

impl std::fmt::Display for Doi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "10.{}/{}", self.organization, self.id)
    }
}
