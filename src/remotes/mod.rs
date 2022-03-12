use itertools::Itertools;
use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Result};
use arxiv::ArxivPaper;
use dblp::DBLPPaper;
use local::{Library, LocalPaper};

use crate::{PaperInfo, Query};
use futures::future::try_join_all;

pub mod arxiv;
pub mod dblp;
pub mod local;

use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaperHit {
    Arxiv(ArxivPaper),
    Dblp(DBLPPaper),
    Local(LocalPaper),
}

impl PaperHit {
    pub fn metadata(&self) -> &PaperInfo {
        match self {
            PaperHit::Arxiv(paper) => paper.metadata(),
            PaperHit::Dblp(paper) => paper.metadata(),
            PaperHit::Local(paper) => paper.metadata(),
        }
    }
}

impl RemoteTag for PaperHit {
    fn remote_tag(&self) -> String {
        match self {
            PaperHit::Arxiv(paper) => paper.remote_tag(),
            PaperHit::Dblp(paper) => paper.remote_tag(),
            PaperHit::Local(paper) => paper.remote_tag(),
        }
    }
}

impl Display for PaperHit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PaperHit::Arxiv(paper) => write!(f, "{}", paper),
            PaperHit::Dblp(paper) => write!(f, "{}", paper),
            PaperHit::Local(paper) => write!(f, "{}", paper),
        }
    }
}

pub trait RemoteTag {
    fn remote_tag(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct Paper(pub Vec<PaperHit>);

impl Paper {
    pub fn new(hits: Vec<PaperHit>) -> Self {
        Paper(hits)
    }

    pub fn hits(&self) -> &[PaperHit] {
        &self.0
    }

    pub fn metadata(&self) -> &PaperInfo {
        &self.0.first().unwrap().metadata()
    }
}

impl PartialEq for Paper {
    fn eq(&self, other: &Paper) -> bool {
        self.0.first().unwrap().metadata() == other.0.first().unwrap().metadata()
    }
}

impl Eq for Paper {}

impl Display for Paper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ", self.metadata())?;
        for hit in self.0.iter() {
            write!(f, "{} ", hit.remote_tag())?;
        }
        write!(f, "")
    }
}

#[async_trait]
pub trait Remote {
    async fn fetch(query: Query) -> Result<Vec<PaperHit>> {
        let response = reqwest::get(Self::get_url(query))
            .await
            .map_err(|err| anyhow!(err))?;
        let body = response.text().await.map_err(|err| anyhow!(err))?;
        // std::fs::write("response.xml", body.clone()).expect("Unable to write file");
        Self::parse_response(&body)
    }

    fn get_url(query: Query) -> String;

    fn parse_response(response: &String) -> Result<Vec<PaperHit>>;
}

pub async fn fetch_all_and_merge(lib: &Library, query: Query) -> Result<Vec<Paper>> {
    let lib = lib.clone();
    let mut handles = Vec::new();
    handles.push(tokio::spawn(arxiv::Arxiv::fetch(query.clone())));
    handles.push(tokio::spawn(dblp::DBLP::fetch(query.clone())));
    handles.push(tokio::spawn(async move {
        Ok(local::get_local_hits(&lib, &query))
    }));

    //let hits = vec![]; //: Vec<Result<Vec<PaperHit>>> = futures::future::try_join_all(handles).await?;
    //merge_papers(hits.into_iter().flatten().collect())
    Ok(vec![])
}

pub fn merge_papers(hits: Vec<PaperHit>) -> Result<Vec<Paper>> {
    let mut papers: Vec<Paper> = hits
        .into_iter()
        .map(|p| (p.metadata().title.normalized(), p))
        .into_group_map()
        .into_iter()
        .map(|(_, v)| Paper::new(v))
        .collect();

    papers.sort_by_key(|r| r.metadata().year.to_owned());
    papers.reverse();
    Ok(papers)
}
