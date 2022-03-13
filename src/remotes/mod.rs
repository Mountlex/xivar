use itertools::Itertools;
use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
};

use anyhow::{anyhow, Result};
use arxiv::ArxivPaper;
use dblp::DBLPPaper;
use local::LocalPaper;

use crate::{PaperInfo, Query};

pub mod arxiv;
pub mod dblp;
pub mod local;

use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaperHit {
    Local(LocalPaper),
    Arxiv(ArxivPaper),
    Dblp(DBLPPaper),
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

pub trait OnlineRemote {
    fn get_url(query: Query) -> String;

    fn parse_response(response: &String) -> Result<Vec<PaperHit>>;
}

#[async_trait]
pub trait Remote {
    async fn fetch_from_remote(&self, query: Query) -> Result<Vec<PaperHit>>;
}

#[async_trait]
impl<R> Remote for R
where
    R: OnlineRemote + std::marker::Send + std::marker::Sync,
{
    async fn fetch_from_remote(&self, query: Query) -> Result<Vec<PaperHit>> {
        let response = reqwest::get(Self::get_url(query))
            .await
            .map_err(|err| anyhow!(err))?;
        let body = response.text().await.map_err(|err| anyhow!(err))?;
        Self::parse_response(&body)
    }
}

pub fn merge_papers<I: Iterator<Item = PaperHit>>(hits: I) -> Result<Vec<Paper>> {
    let mut papers: Vec<Paper> = hits
        .map(|p| (p.metadata().title.normalized(), p))
        .into_group_map()
        .into_iter()
        .map(|(_, v)| Paper::new(v))
        .collect();

    papers.sort_by_key(|r| r.metadata().year.to_owned());
    papers.reverse();
    Ok(papers)
}

pub fn merge_to_papers<I: Iterator<Item = PaperHit>>(
    papers: Vec<Paper>,
    hits: I,
) -> Result<Vec<Paper>> {
    let mut papers: Vec<Paper> = papers
        .into_iter()
        .flat_map(|p| p.0)
        .chain(hits)
        .map(|p| (p.metadata().title.normalized(), p))
        .into_group_map()
        .into_iter()
        .map(|(_, mut v)| {
            v.sort_by(|a, b| match (a, b) {
                (PaperHit::Local(_), PaperHit::Local(_)) => Ordering::Equal,
                (PaperHit::Arxiv(_), PaperHit::Arxiv(_)) => Ordering::Equal,
                (PaperHit::Dblp(_), PaperHit::Dblp(_)) => Ordering::Equal,
                (PaperHit::Local(_), _) => Ordering::Less,
                (PaperHit::Arxiv(_), PaperHit::Dblp(_)) => Ordering::Less,
                (PaperHit::Arxiv(_), PaperHit::Local(_)) => Ordering::Greater,
                (PaperHit::Dblp(_), _) => Ordering::Greater,
            });
            Paper::new(v)
        })
        .collect();
    papers.sort_by_key(|r| r.metadata().year.to_owned());
    papers.reverse();
    Ok(papers)
}
