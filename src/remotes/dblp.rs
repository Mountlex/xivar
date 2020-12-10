use anyhow::{anyhow, Result};

use crate::{ArxivIdentifier, Doi, Identifier, Paper, PaperUrl, Query};

use super::Remote;

pub struct DBLP;

impl Remote for DBLP {
    fn get_url(query: Query) -> String {
        format!(
            "https://dblp.org/search/publ/api?q={}&h={}",
            query.terms.map(|t| t.join("+")).unwrap_or_default(),
            query.max_hits
        )
    }

    fn parse_response(response: &String) -> Result<Vec<Paper>> {
        let doc = roxmltree::Document::parse(response)?;
        let hits = doc
            .descendants()
            .find(|n| n.has_tag_name("hits"))
            .ok_or(anyhow!("No results!"))?;
        let _number_of_hits = hits.attribute("total").unwrap().parse::<u32>();

        let papers: Vec<Paper> = hits
            .children()
            .filter_map(|hit| {
                hit.descendants()
                    .find(|n| n.has_tag_name("info"))
                    .map(|info| {
                        let title = info
                            .descendants()
                            .find(|n| n.has_tag_name("title"))
                            .unwrap()
                            .text()
                            .unwrap()
                            .to_owned();
                        let year = info
                            .descendants()
                            .find(|n| n.has_tag_name("year"))
                            .unwrap()
                            .text()
                            .unwrap()
                            .to_owned();
                        let authors: Vec<String> = info
                            .descendants()
                            .filter(|n| n.has_tag_name("author"))
                            .map(|a| a.text().unwrap().trim().to_owned())
                            .collect();

                        let url_string = info
                            .children()
                            .find(|n| n.has_tag_name("ee"))
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
                        }
                        .unwrap_or_else(|| Identifier::Custom(title.replace(r"\s", "_")));

                        Paper {
                            id,
                            authors,
                            title,
                            year,
                            url,
                            local_path: None,
                        }
                    })
            })
            .collect();

        Ok(papers)
    }
}
