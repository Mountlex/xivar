use anyhow::{anyhow, Result};

use crate::{ArxivIdentifier, Identifier, Paper, PaperUrl, Query};

use super::Remote;

pub struct Arxiv;

impl Remote for Arxiv {
    fn get_url(query: Query) -> String {
        format!(
            "http://export.arxiv.org/api/query?search_query={}&max_results={}",
            query.terms.map(|t| t.join("+")).unwrap_or_default(),
            query.max_hits
        )
    }

    fn parse_response(response: &String) -> Result<Vec<Paper>> {
        let doc = roxmltree::Document::parse(response)?;
        let feed = doc
            .descendants()
            .find(|n| n.has_tag_name("feed"))
            .ok_or(anyhow!("No results!"))?;

        let papers: Vec<Paper> = feed
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
                let _summary = entry
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
                    .split("-")
                    .collect::<Vec<&str>>()
                    .first()
                    .unwrap()
                    .to_owned()
                    .to_owned();
                let authors: Vec<String> = entry
                    .children()
                    .filter(|n| n.has_tag_name("author"))
                    .map(|a| {
                        a.children()
                            .find(|c| c.has_tag_name("name"))
                            .unwrap()
                            .text()
                            .unwrap()
                            .trim()
                            .to_owned()
                    })
                    .collect();

                let url_string = entry
                    .children()
                    .find(|n| n.has_tag_name("id"))
                    .map(|n| n.text().unwrap().to_owned())
                    .unwrap_or("None".to_owned());
                let url = PaperUrl::new(url_string);
                let id = ArxivIdentifier::parse_string(&url.raw())
                    .ok()
                    .map(Identifier::Arxiv)
                    .unwrap();

                Paper {
                    id,
                    authors,
                    title,
                    year,
                    url,
                    local_path: None,
                }
            })
            .collect();

        Ok(papers)
    }
}
