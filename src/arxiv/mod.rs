use std::{io::Write};

use anyhow::{Result, anyhow};
use arxiv::ArxivQueryBuilder;
use std::path::PathBuf;

use crate::store::Paper;


pub async fn get_online_results(query: &Vec<String>) -> Result<Vec<Paper>> {
    let query = ArxivQueryBuilder::new()
        .search_query(query.join("+").as_ref())
        .start(0)
        .max_results(30)
        //.sort_by("submittedDate")
        //.sort_order("descending")
        .build();
    let results = arxiv::fetch_arxivs(query).await?;
    Ok(results
        .into_iter()
        .filter_map(|arxiv| Paper::from_arxiv(arxiv).ok())
        .collect())
}


pub async fn download_pdf(paper: &Paper, out_path: &PathBuf) -> Result<()> {
    let mut response = surf::get(&paper.pdf_url).await.map_err(|err| anyhow!(err))?;
    let body = response.body_bytes().await.map_err(|err| anyhow!(err))?;
    let mut file = std::fs::File::create(
        out_path
            .with_extension("pdf"),
    )?;
    file.write_all(&body)?;
    Ok(())
}
