use super::Command;
use crate::config;
use crate::fzf;
use crate::store::{Library, Paper, Query};
use anyhow::{Context, Result, anyhow};
use arxiv::{Arxiv, ArxivQueryBuilder};
use clap::Clap;
use futures::executor::block_on;
use std::{path::PathBuf, io::Write};


#[derive(Clap, Debug)]
pub struct Search {
    query: Vec<String>,
}

impl Command for Search {
    fn run(&self) -> Result<()> {
        let mut fzf = fzf::Fzf::new()?;
        let handle = fzf.stdin();
        let query = self.query.clone();

        let (mut store_results, mut online_results): (Vec<Paper>, Vec<Paper>) = block_on(async {
            let store_handle = get_store_results(query.clone());
            let online_handle = get_online_results(query);
            futures::try_join!(store_handle, online_handle)
        })?;

        let mut results = vec![];
        results.append(&mut store_results);
        results.append(&mut online_results);
        for result in results.iter() {
            writeln!(handle, "{}", result).context("could not write to fzf")?;
        }
        let sel = fzf.wait_select()?;
        let selection: Vec<_> = sel.split("[").collect();

        let paper = results.iter().find(|paper| paper.title.trim() == selection[0].trim()).unwrap();
        block_on(download_pdf(paper.clone(), PathBuf::from(".")))
    }
}

async fn download_pdf(paper: Paper, out_path: PathBuf) -> Result<()> {
    let mut response = surf::get(paper.pdf_url).await.map_err(|err| anyhow!(err))?;
    let body = response.body_bytes().await.map_err(|err| anyhow!(err))?;
    let mut file = std::fs::File::create(out_path.with_file_name(paper.id.as_str()).with_extension("pdf"))?;
    file.write_all(&body)?;
    Ok(())
}

async fn get_store_results(query: Vec<String>) -> Result<Vec<Paper>> {
    let data_dir = config::xivar_data_dir()?;
    let lib = Library::open(&data_dir)?;
    Ok(lib
        .into_iter_matches(Query::Full(query.join(" ")))
        .into_iter()
        .filter(|paper| paper.exists())
        .collect())
}

async fn get_online_results(query: Vec<String>) -> Result<Vec<Paper>> {
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
        .map(|arxiv| Paper::from(arxiv))
        .collect())
}
