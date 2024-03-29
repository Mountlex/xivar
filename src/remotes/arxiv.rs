use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Result};
use console::style;
use itertools::Itertools;

use crate::{
    query::QueryTerm, ArxivIdentifier, Identifier, PaperInfo, PaperTitle, PaperUrl, Query, Venue,
};

use super::{OnlineRemote, PaperHit};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArxivPaper {
    metadata: PaperInfo,
    pub ee: PaperUrl,
}

impl ArxivPaper {
    pub fn metadata(&self) -> &PaperInfo {
        &self.metadata
    }

    pub fn download_url(&self) -> PaperUrl {
        PaperUrl::new(format!(
            "https://arxiv.org/pdf/{}.pdf",
            self.metadata.id.as_ref().unwrap()
        ))
    }

    pub fn remote_tag(&self) -> String {
        style(format!("arXiv({})", self.metadata().year))
            .yellow()
            .bold()
            .to_string()
    }
}

impl Display for ArxivPaper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.metadata, self.remote_tag())
    }
}

#[derive(Clone, Debug)]
pub struct Arxiv;

impl OnlineRemote for Arxiv {
    fn get_url(query: &Query, max_hits: usize) -> String {
        format!(
            "http://export.arxiv.org/api/query?search_query={}&max_results={}",
            query
                .into_iter()
                .map(|t| {
                    match t {
                        QueryTerm::Exact(q) => q.to_string(),
                        QueryTerm::Prefix(q) => q.to_string(),
                    }
                })
                .join("+AND+"),
            max_hits
        )
    }

    fn parse_response(response: &str) -> Result<Vec<PaperHit>> {
        let doc = roxmltree::Document::parse(response)?;
        let feed = doc
            .descendants()
            .find(|n| n.has_tag_name("feed"))
            .ok_or_else(|| anyhow!("No results!"))?;

        let papers: Vec<PaperHit> = feed
            .children()
            .filter(|entry| entry.has_tag_name("entry"))
            .map(|entry| {
                let title = entry
                    .children()
                    .find(|n| n.has_tag_name("title"))
                    .unwrap()
                    .text()
                    .unwrap()
                    .to_owned();
                let summary = entry
                    .children()
                    .find(|n| n.has_tag_name("summary"))
                    .unwrap()
                    .text()
                    .unwrap()
                    .to_owned();
                let year = entry
                    .children()
                    .find(|n| n.has_tag_name("published"))
                    .unwrap()
                    .text()
                    .unwrap()
                    .split('-')
                    .collect::<Vec<&str>>()
                    .first()
                    .unwrap()
                    .to_owned()
                    .to_owned();
                let authors: Vec<String> = entry
                    .children()
                    .filter(|n| n.has_tag_name("author"))
                    .map(|a| {
                        let raw = a
                            .children()
                            .find(|c| c.has_tag_name("name"))
                            .unwrap()
                            .text()
                            .unwrap();
                        let re = regex::Regex::new(r"\d{4}").unwrap();
                        re.replace_all(raw, "").trim().to_owned()
                    })
                    .collect();

                let url_string = entry
                    .children()
                    .find(|n| n.has_tag_name("id"))
                    .map(|n| n.text().unwrap().to_owned())
                    .unwrap_or_else(|| "None".to_owned());
                let ee = PaperUrl::new(url_string);
                let id = ArxivIdentifier::parse_string(&ee.raw())
                    .ok()
                    .map(Identifier::Arxiv)
                    .unwrap();

                let paper = PaperInfo {
                    id: Some(id),
                    authors,
                    venue: Venue::Journal("CoRR".to_owned()),
                    title: PaperTitle::new(title),
                    year,
                    summary: Some(summary),
                };
                PaperHit::Arxiv(ArxivPaper {
                    metadata: paper,
                    ee,
                })
            })
            .collect();

        Ok(papers)
    }

    fn name(&self) -> String {
        style("arXiv").yellow().bold().to_string()
    }
}
