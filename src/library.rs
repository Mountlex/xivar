use console::style;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    fs, io,
};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::{NamedTempFile, PersistError};

pub use crate::Query;
use crate::{PaperInfo, PaperUrl};
use anyhow::{bail, Context, Result};
use bincode::Options;

#[derive(Debug)]
pub enum LibReq {
    Save {
        paper: LocalPaper,
    },
    Query {
        res_channel: tokio::sync::oneshot::Sender<Vec<LocalPaper>>,
        query: Query,
        max_hits: usize,
    },
}

#[derive(Debug)]
pub enum LoadingResult {
    Success,
    Failure(String),
}

pub async fn lib_manager_fut(
    data_dir: PathBuf,
    mut req_recv: tokio::sync::mpsc::Receiver<LibReq>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    loading_tx: tokio::sync::mpsc::Sender<LoadingResult>,
) {
    log::info!("Load library...");

    match Library::open(&data_dir) {
        Ok(mut lib) => {
            log::info!("Library loaded! {} local entries.", lib.size());
            loading_tx.send(LoadingResult::Success).await.unwrap();
            loop {
                tokio::select! {
                    req = req_recv.recv() => {
                        if let Some(req) = req {
                                match req {
                                    LibReq::Save { paper } => {
                                        lib.add(paper);
                                    }
                                    LibReq::Query { res_channel, query, max_hits } => {
                                        let results = lib.iter_matches(&query).cloned().take(max_hits).collect();
                                        res_channel.send(results).unwrap();
                                    }
                                }
                        }
                    }
                    _ = shutdown_rx.recv() => break
                }
            }
        }
        Err(err) => {
            loading_tx
                .send(LoadingResult::Failure(err.to_string()))
                .await
                .unwrap();
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct LocalPaper {
    pub metadata: PaperInfo,
    pub location: PathBuf,
    pub ees: Vec<PaperUrl>,
}

impl LocalPaper {
    pub fn exists(&self) -> bool {
        Path::new(&self.location).exists()
    }
    pub fn metadata(&self) -> &PaperInfo {
        &self.metadata
    }
    pub fn remote_tag(&self) -> String {
        style(format!(
            "Local({} {})",
            self.metadata().year,
            self.metadata().venue
        ))
        .red()
        .bold()
        .to_string()
    }
}

impl Display for LocalPaper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.metadata, self.remote_tag())
    }
}

#[derive(Debug, Clone)]
pub struct Library {
    papers: Vec<LocalPaper>,
    modified: bool,
    data_dir: PathBuf,
}

impl Library {
    pub const CURRENT_VERSION: LibraryVersion = LibraryVersion(1);

    pub fn open<P: Into<PathBuf>>(data_dir: P) -> Result<Library> {
        let data_dir = data_dir.into();
        let path = Self::get_path(&data_dir);

        let buffer = match fs::read(&path) {
            Ok(buffer) => buffer,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                fs::create_dir_all(&data_dir).with_context(|| {
                    format!("unable to create data directory: {}", path.display())
                })?;
                return Ok(Library {
                    papers: Vec::new(),
                    modified: false,
                    data_dir,
                });
            }
            Err(e) => {
                Err(e).with_context(|| format!("could not read from store: {}", path.display()))?
            }
        };

        let deserializer = &mut bincode::options().with_fixint_encoding();

        let version_size = deserializer
            .serialized_size(&Self::CURRENT_VERSION)
            .unwrap() as _;

        let (buffer_version, buffer_papers) = buffer.split_at(version_size);

        let version = deserializer
            .deserialize(buffer_version)
            .with_context(|| format!("could not deserialize store version: {}", path.display()))?;

        let papers = match version {
            Self::CURRENT_VERSION => deserializer
                .deserialize(buffer_papers)
                .with_context(|| format!("could not deserialize store: {}", path.display()))?,
            version => bail!(
                "unsupported store version, got={}, supported={}: {}",
                version.0,
                Self::CURRENT_VERSION.0,
                path.display()
            ),
        };

        Ok(Library {
            papers,
            modified: false,
            data_dir,
        })
    }

    pub fn save(&mut self) -> Result<()> {
        if !self.modified {
            return Ok(());
        }

        log::warn!("Saving library...");

        let (buffer, buffer_size) = (|| -> bincode::Result<_> {
            let version_size = bincode::serialized_size(&Self::CURRENT_VERSION)?;
            let papers_size = bincode::serialized_size(&self.papers)?;

            let buffer_size = version_size + papers_size;
            let mut buffer = Vec::with_capacity(buffer_size as _);

            bincode::serialize_into(&mut buffer, &Self::CURRENT_VERSION)?;
            bincode::serialize_into(&mut buffer, &self.papers)?;

            Ok((buffer, buffer_size))
        })()
        .context("could not serialize store")?;

        let mut file = NamedTempFile::new_in(&self.data_dir).with_context(|| {
            format!(
                "could not create temporary store in: {}",
                self.data_dir.display()
            )
        })?;

        let _ = file.as_file().set_len(buffer_size);
        file.write_all(&buffer).with_context(|| {
            format!(
                "could not write to temporary store: {}",
                file.path().display()
            )
        })?;

        let path = Self::get_path(&self.data_dir);
        persist(file, &path)
            .with_context(|| format!("could not replace store: {}", path.display()))?;

        log::warn!("... done!");

        self.modified = false;
        Ok(())
    }

    pub fn add(&mut self, paper: LocalPaper) {
        match self
            .papers
            .iter_mut()
            .find(|p| p.metadata() == paper.metadata())
        {
            None => {
                self.papers.push(paper);
            }
            Some(p) => {
                p.location = paper.location;
            }
        };
        self.modified = true;
    }

    // pub fn remove(&mut self, paper: &Paper) -> bool {
    //     if let Some(idx) = self.papers.iter().position(|p| p == paper) {
    //         self.papers.swap_remove(idx);
    //         self.modified = true;
    //         return true;
    //     }

    //     false
    // }

    pub fn iter_matches<'a>(&'a self, query: &'a Query) -> impl Iterator<Item = &'a LocalPaper> {
        self.papers
            .iter()
            .filter(move |copy| copy.metadata.matches(query))
    }

    pub fn size(&self) -> usize {
        self.papers.len()
    }

    #[allow(dead_code)]
    pub fn find_paper_by_path<'a>(&'a self, path: &Path) -> Option<&'a LocalPaper> {
        self.papers.iter().find(|paper| paper.location == path)
    }

    pub fn clean(&mut self) -> Vec<LocalPaper> {
        let mut to_remove: Vec<usize> = vec![];
        for (idx, _) in self
            .papers
            .iter()
            .filter(|paper| !paper.exists())
            .enumerate()
        {
            to_remove.push(idx);
        }
        to_remove.sort_unstable();
        to_remove.reverse();
        let removed = to_remove
            .iter()
            .map(|&i| self.papers.swap_remove(i))
            .collect();
        self.modified = true;
        removed
    }

    pub fn clear(&mut self) -> Vec<LocalPaper> {
        let removed = self.papers.drain(..).collect();
        self.modified = true;
        removed
    }

    fn get_path<P: AsRef<Path>>(data_dir: P) -> PathBuf {
        data_dir.as_ref().join("lib.db")
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            println!("Error: {}", e)
        }
    }
}

fn persist<P: AsRef<Path>>(file: NamedTempFile, path: P) -> Result<(), PersistError> {
    file.persist(&path)?;
    Ok(())
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct LibraryVersion(pub u32);
