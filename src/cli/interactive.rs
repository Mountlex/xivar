use std::path::{Path, PathBuf};

use super::{
    actions::{self, Action},
    util, Command,
};
use crate::remotes;
use anyhow::Result;
use async_std::task;
use clap::Parser;
use dialoguer::Input;
use remotes::{
    local::{Library, LocalPaper},
    PaperHit,
};
use std::io::{stdout, Read, Write};
use termion::raw::IntoRawMode;
use termion::{event::Key, input::TermRead};

#[derive(Parser, Debug)]
#[clap(about = "Search remotes and your local library")]
pub struct Interactive {}

impl Command for Interactive {
    fn run(&self) -> Result<()> {
        let mut stdin = termion::async_stdin().keys();
        let (sender, receiver) = async_std::channel::bounded(10);

        let child = task::spawn(async move {
            while let Ok(text) = receiver.recv().await {
                println!("Text: {}", text);
            }
        });

        loop {
            let k = stdin.next();

            if let Some(Ok(key)) = k {
                match key {
                    Key::Char(c) => sender.try_send(c),
                    _ => Ok(()),
                };
            }
        }

        Ok(())
    }
}
