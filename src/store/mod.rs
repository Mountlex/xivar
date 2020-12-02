mod paper;
mod query;
mod identifier;

use serde::{Deserialize, Serialize};
use std::{fs, io};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::{NamedTempFile, PersistError};

use anyhow::{bail, Context, Result};
use bincode::Options;
pub use paper::{MatchByTitle, Paper, PaperCopy, PaperUrl};
pub use identifier::*;
pub use query::Query;

pub fn get_store_results(query: &Vec<String>, lib: &Library) -> Result<Vec<PaperCopy>> {
    Ok(lib
        .iter_matches(Query::Full(query.as_slice()))
        .cloned()
        .collect())
}

#[derive(Debug)]
pub struct Library {
    papers: Vec<PaperCopy>,
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

        self.modified = false;
        Ok(())
    }

    pub fn add(&mut self, location: &PathBuf, paper: Paper) {
        match self.papers.iter().find(|&p| p.paper == paper) {
            None => self.papers.push(PaperCopy {
                paper,
                location: location.clone(),
            }),
            Some(_) => {}
        };

        self.modified = true;
    }

    pub fn remove(&mut self, paper: &Paper) -> bool {
        if let Some(idx) = self.papers.iter().position(|copy| &copy.paper == paper) {
            self.papers.swap_remove(idx);
            self.modified = true;
            return true;
        }

        false
    }

    pub fn iter_matches<'a>(&'a self, query: Query<'a>) -> impl Iterator<Item = &'a PaperCopy> {
        self.papers
            .iter()
            .filter(move |copy| copy.paper.matches(query.clone()))
    }

    pub fn clean(&mut self) -> Vec<PaperCopy> {
        let mut to_remove: Vec<usize> = vec![];
        for (idx, _) in self
            .papers
            .iter()
            .filter(|paper| !paper.exists())
            .enumerate()
        {
            to_remove.push(idx);
        }
        to_remove.sort();
        to_remove.reverse();
        let removed = to_remove
            .iter()
            .map(|&i| self.papers.swap_remove(i))
            .collect();
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
