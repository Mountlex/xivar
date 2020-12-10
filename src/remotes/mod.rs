use std::future::Future;

use anyhow::{anyhow, Result};
use dblp::DBLP;

use crate::{Paper, Query};
use async_std::prelude::*;

pub mod arxiv;
pub mod dblp;

use async_trait::async_trait;

#[async_trait]
pub trait Remote {
    async fn fetch(query: Query) -> Result<Vec<Paper>> {
        let mut response = surf::get(Self::get_url(query))
            .await
            .map_err(|err| anyhow!(err))?;
        let body = response.body_string().await.map_err(|err| anyhow!(err))?;
        // std::fs::write("response.xml", body.clone()).expect("Unable to write file");
        Self::parse_response(&body)
    }

    fn get_url(query: Query) -> String;

    fn parse_response(response: &String) -> Result<Vec<Paper>>;
}

pub async fn fetch_all(query: Query) -> impl Future<Output = Result<(Vec<Paper>, Vec<Paper>)>> {
    arxiv::Arxiv::fetch(query.clone()).try_join(DBLP::fetch(query.clone()))
}

pub async fn fetch_all_and_merge(query: Query) -> Result<Vec<Paper>> {
    let (mut a, b): (Vec<Paper>, Vec<Paper>) = arxiv::Arxiv::fetch(query.clone())
        .try_join(dblp::DBLP::fetch(query.clone()))
        .await?;

    for paper in b {
        if !a.contains(&paper) {
            a.push(paper)
        }
    }
    a.sort_by_key(|paper| paper.year.clone());
    a.reverse();
    Ok(a)
}
