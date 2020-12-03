use anyhow::{anyhow, Result};

use crate::store::{ArxivIdentifier, Doi, Identifier, Paper, PaperUrl, Preprint};

use super::RequestString;

pub async fn fetch_publication_query(terms: &[String], max_hits: u32) -> Result<Vec<Paper>> {
    let query = Query::Publication(QueryContent { terms, max_hits });
    let mut response = surf::get(query.query_url())
        .await
        .map_err(|err| anyhow!(err))?;
    let body = response.body_string().await.map_err(|err| anyhow!(err))?;
    std::fs::write("response.xml", body.clone()).expect("Unable to write file");
    parse_publ_response(&body)
}

enum Query<'a> {
    Publication(QueryContent<'a>),
    Author(QueryContent<'a>),
    Venue(QueryContent<'a>),
}

impl<'a> RequestString for Query<'a> {
    fn query_url(&self) -> String {
        match self {
            Query::Publication(content) => {
                format!(
                    "https://dblp.org/search/publ/api?{}",
                    content.get_query_string()
                )
            }
            Query::Author(content) => {
                format!(
                    "https://dblp.org/search/author/api?{}",
                    content.get_query_string()
                )
            }
            Query::Venue(content) => {
                format!(
                    "https://dblp.org/search/venue/api?{}",
                    content.get_query_string()
                )
            }
        }
    }
}

struct QueryContent<'a> {
    terms: &'a [String],
    max_hits: u32,
}

impl<'a> QueryContent<'a> {
    fn get_query_string(&self) -> String {
        format!("q={}&h={}", self.terms.join("+"), self.max_hits)
    }
}

fn parse_publ_response(response: &str) -> Result<Vec<Paper>> {
    let doc = roxmltree::Document::parse(response)?;
    let hits = doc
        .descendants()
        .find(|n| n.has_tag_name("hits"))
        .ok_or(anyhow!("No results!"))?;
    let number_of_hits = hits.attribute("total").unwrap().parse::<u32>();

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
                    let id = if let Some(doi) = info.children().find(|n| n.has_tag_name("doi")) {
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
                    }
                })
        })
        .collect();

    Ok(papers)
}
