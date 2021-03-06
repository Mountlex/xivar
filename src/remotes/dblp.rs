use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Result};
use console::style;

use crate::{ArxivIdentifier, Doi, Identifier, PaperInfo, PaperTitle, PaperUrl, Query};

use super::{PaperHit, Remote, RemoteTag};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DBLPPaper {
    metadata: PaperInfo,
    pub url: PaperUrl,
    pub ee: PaperUrl,
}

impl DBLPPaper {
    pub fn metadata(&self) -> &PaperInfo {
        &self.metadata
    }
}

impl RemoteTag for DBLPPaper {
    fn remote_tag(&self) -> String {
        style(format!(
            "DBLP({} {})",
            self.metadata().year,
            self.metadata().venue
        ))
        .cyan()
        .bold()
        .to_string()
    }
}

impl Display for DBLPPaper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.metadata, self.remote_tag())
    }
}

pub struct DBLP;

impl Remote for DBLP {
    fn get_url(query: Query) -> String {
        if let Some(max_hits) = query.max_hits {
            format!(
                "https://dblp.org/search/publ/api?q={}&h={}",
                query.terms.map(|t| t.join("+")).unwrap_or_default(),
                max_hits
            )
        } else {
            format!(
                "https://dblp.org/search/publ/api?q={}",
                query.terms.map(|t| t.join("+")).unwrap_or_default()
            )
        }
    }

    fn parse_response(response: &String) -> Result<Vec<PaperHit>> {
        let doc = roxmltree::Document::parse(response)?;
        let hits = doc
            .descendants()
            .find(|n| n.has_tag_name("hits"))
            .ok_or(anyhow!("No results!"))?;
        let _number_of_hits = hits.attribute("total").unwrap().parse::<u32>();

        let papers: Vec<PaperHit> = hits
            .children()
            .filter_map(|hit| {
                hit.descendants()
                    .find(|n| n.has_tag_name("info"))
                    .map(|info| {
                        let title = info
                            .children()
                            .find(|n| n.has_tag_name("title"))
                            .unwrap()
                            .text()
                            .unwrap()
                            .to_owned();
                        let venue = info
                            .children()
                            .find(|n| n.has_tag_name("venue"))
                            .map(|v| v.text().unwrap().to_owned())
                            .unwrap_or_default();
                        let year = info
                            .children()
                            .find(|n| n.has_tag_name("year"))
                            .unwrap()
                            .text()
                            .unwrap()
                            .to_owned();
                        let authors: Vec<String> = info
                            .descendants()
                            .filter(|n| n.has_tag_name("author"))
                            .map(|a| {
                                let raw = a.text().unwrap();
                                let re = regex::Regex::new(r"\d{4}").unwrap();
                                re.replace_all(raw, "").trim().to_owned()
                            })
                            .collect();

                        let ee_string = info
                            .children()
                            .find(|n| n.has_tag_name("ee"))
                            .map(|n| n.text().unwrap().to_owned())
                            .unwrap_or("None".to_owned());
                        let ee = PaperUrl::new(ee_string);
                        let url_string = info
                            .children()
                            .find(|n| n.has_tag_name("url"))
                            .map(|n| n.text().unwrap().to_owned())
                            .unwrap_or("None".to_owned());
                        let url = PaperUrl::new(url_string);
                        let id = if let Some(doi) = info.children().find(|n| n.has_tag_name("doi"))
                        {
                            let doi_string = doi.text().unwrap();
                            Doi::parse_doi(doi_string).ok().map(Identifier::Doi)
                        } else if url.raw().contains("arxiv") {
                            ArxivIdentifier::parse_string(&url.raw())
                                .ok()
                                .map(Identifier::Arxiv)
                        } else {
                            None
                        };

                        let paper = PaperInfo {
                            id,
                            authors,
                            venue,
                            title: PaperTitle::new(title),
                            year,
                        };
                        PaperHit::Dblp(DBLPPaper {
                            metadata: paper,
                            url,
                            ee,
                        })
                    })
            })
            .collect();

        Ok(papers)
    }
}
