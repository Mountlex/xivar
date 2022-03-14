use anyhow::{anyhow, Result};

use crate::{PaperHit, Query};

pub mod arxiv;
pub mod dblp;
pub mod local;

use async_trait::async_trait;

pub trait OnlineRemote {
    fn get_url(query: Query) -> String;

    fn parse_response(response: &String) -> Result<Vec<PaperHit>>;
}

pub struct FetchResult {
    pub hits: Vec<PaperHit>,
}

#[async_trait]
pub trait Remote {
    async fn fetch_from_remote(&self, query: Query) -> Result<FetchResult>;
}

#[async_trait]
impl<R> Remote for R
where
    R: OnlineRemote + std::marker::Send + std::marker::Sync,
{
    async fn fetch_from_remote(&self, query: Query) -> Result<FetchResult> {
        let response = reqwest::get(Self::get_url(query))
            .await
            .map_err(|err| anyhow!(err))?;
        let body = response.text().await.map_err(|err| anyhow!(err))?;
        Ok(FetchResult {
            hits: Self::parse_response(&body)?,
        })
    }
}
