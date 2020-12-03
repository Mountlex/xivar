use anyhow::{bail, Context, Result};

use std::process::{Child, ChildStdin, Command, Stdio};

pub struct Fzf {
    child: Child,
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
