use anyhow::{anyhow, Result};

use crate::{PaperHit, Query};

pub mod arxiv;
pub mod dblp;
pub mod local;

use async_trait::async_trait;

pub trait OnlineRemote {
    fn get_url(query: &Query, max_hits: usize) -> String;

    fn parse_response(response: &str) -> Result<Vec<PaperHit>>;

    fn name(&self) -> String;
}

pub struct FetchResult {
    pub query: Query,
    pub hits: Vec<PaperHit>,
}

#[async_trait]
pub trait Remote {
    async fn fetch_from_remote(&self, query: Query, max_hits: usize) -> Result<FetchResult>;

    fn name(&self) -> String;
}

#[async_trait]
impl<R> Remote for R
where
    R: OnlineRemote + std::marker::Send + std::marker::Sync,
{
    async fn fetch_from_remote(&self, query: Query, max_hits: usize) -> Result<FetchResult> {
        let response = reqwest::get(Self::get_url(&query, max_hits))
            .await
            .map_err(|err| anyhow!(err))?;
        let body = response.text().await.map_err(|err| anyhow!(err))?;
        Ok(FetchResult {
            query,
            hits: Self::parse_response(&body)?,
        })
    }

    fn name(&self) -> String {
        self.name()
    }
}
