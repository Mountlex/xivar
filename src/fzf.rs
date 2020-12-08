use anyhow::{bail, Context, Result};

use async_std::sync::{Arc, Mutex};
use async_std::task;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::{future::Future, io::Write};

pub struct Fzf<E> {
    data: Arc<Mutex<EnumeratedStdin<E>>>,
}

#[derive(Debug)]
struct EnumeratedStdin<E> {
    child: Child,
    list: Vec<E>,
}

impl<E: std::fmt::Display + Eq> EnumeratedStdin<E> {
    fn new(child: Child) -> EnumeratedStdin<E> {
        EnumeratedStdin {
            child,
            list: vec![],
        }
    }
    pub fn stdin(&mut self) -> &mut ChildStdin {
        self.child.stdin.as_mut().unwrap()
    }

    fn write(&mut self, entry: E) {
        if !self.list.contains(&entry) {
            let len = self.list.len();
            writeln!(self.stdin(), "{} {}", len, &entry).unwrap();
            self.list.push(entry);
        }
    }
}

impl<E: std::fmt::Display + Eq + Clone> Fzf<E> {
    pub fn new() -> Result<Self> {
        let mut command = Command::new("fzf");
        command.arg("--ansi");
        command.stdin(Stdio::piped()).stdout(Stdio::piped());

        let child = command.spawn().context("could not launch fzf")?;

        Ok(Fzf {
            data: Arc::new(Mutex::new(EnumeratedStdin::new(child))),
        })
    }

    pub fn write_all(&mut self, entries: Vec<E>) {
        task::block_on(async {
            let mut data_handle = self.data.lock().await;
            for entry in entries {
                data_handle.write(entry);
            }
        });
    }

    pub async fn fetch_and_write<F>(&self, fetch: F) -> Result<usize>
    where
        F: Future<Output = Result<Vec<E>>>,
    {
        let results = task::block_on(fetch)?;
        let num_results = results.len();
        let mut data_handle = self.data.lock().await;
        for entry in results {
            data_handle.write(entry);
        }
        Ok(num_results)
    }

    pub fn wait_for_selection(self) -> Result<E> {
        if let Ok(mutex) = Arc::try_unwrap(self.data) {
            let mut data = mutex.into_inner();

            let output = data
                .child
                .wait_with_output()
                .context("wait failed on fzf")?;

            match output.status.code() {
                // normal exit
                Some(0) => {
                    let output = String::from_utf8(output.stdout).context("Invalid encoding!")?;
                    let selection: Vec<_> = output.split(" ").collect();
                    let idx = selection.first().unwrap().to_owned().parse::<usize>()?;
                    let paper = data.list.remove(idx);
                    Ok(paper.clone())
                }

                // no match
                Some(1) => bail!("no match found"),

                // error
                Some(2) => bail!("fzf returned an error"),

                // terminated by a signal
                //Some(code @ 130) => bail!(SilentExit { code }),
                Some(128..=254) | None => bail!("fzf was terminated"),

                // unknown
                _ => bail!("fzf returned an unknown error"),
            }
        } else {
            bail!("Could not go out of mutex")
        }
    }
}
