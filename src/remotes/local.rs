use async_trait::async_trait;
use console::style;

use crate::library::{LibReq, LocalPaper};
pub use crate::Query;
use anyhow::Result;

use super::{FetchResult, PaperHit, Remote};

#[derive(Clone, Debug)]
pub struct LocalRemote {
    query_sender: tokio::sync::mpsc::Sender<LibReq>,
}

impl LocalRemote {
    pub fn with_sender(query_sender: tokio::sync::mpsc::Sender<LibReq>) -> Self {
        LocalRemote { query_sender }
    }
}

#[async_trait]
impl Remote for LocalRemote {
    async fn fetch_from_remote(&self, query: Query, max_hits: usize) -> Result<FetchResult> {
        let (res_sender, res_recv) = tokio::sync::oneshot::channel::<Vec<LocalPaper>>();
        self.query_sender
            .send(LibReq::Query {
                res_channel: res_sender,
                query: query.clone(),
                max_hits,
            })
            .await?;
        let results = res_recv.await.map_err(|err| anyhow::anyhow!(err))?;
        Ok(FetchResult {
            query,
            hits: results
                .into_iter()
                .map(PaperHit::Local)
                .collect::<Vec<PaperHit>>(),
        })
    }

    fn name(&self) -> String {
        style("Local").red().bold().to_string()
    }
}
