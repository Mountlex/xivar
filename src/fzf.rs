use anyhow::{bail, Context, Result};

use async_std::sync::{Arc, Mutex};
use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};

pub struct Fzf {
    child: Child,
}

pub struct EnumerateStdinHandle<'a, E> {
    stdin_handle: &'a mut ChildStdin,
    list: Vec<E>,
}

impl<'a, E: std::fmt::Display> EnumerateStdinHandle<'a, E> {
    fn new(stdin_handle: &'a mut ChildStdin) -> EnumerateStdinHandle<'a, E> {
        EnumerateStdinHandle {
            stdin_handle: stdin_handle,
            list: vec![],
        }
    }

    fn add(&mut self, entry: E) {
        writeln!(self.stdin_handle, "{} {}", self.list.len(), &entry);
        self.list.push(entry);
    }
}

impl Fzf {
    pub fn new() -> Result<Self> {
        let mut command = Command::new("fzf");
        command.arg("--ansi");
        command.stdin(Stdio::piped()).stdout(Stdio::piped());

        Ok(Fzf {
            child: command.spawn().context("could not launch fzf")?,
        })
    }

    pub fn stdin(&mut self) -> &mut ChildStdin {
        self.child.stdin.as_mut().unwrap()
    }

    pub fn wait_select(self) -> Result<String> {
        let output = self
            .child
            .wait_with_output()
            .context("wait failed on fzf")?;

        match output.status.code() {
            // normal exit
            Some(0) => String::from_utf8(output.stdout).context("invalid unicode in fzf output"),

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
    }
}
