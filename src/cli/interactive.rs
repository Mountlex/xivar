use crate::{
    remotes::{self, Remote},
    Query,
};
use anyhow::Result;
use async_std::task;
use clap::Parser;
use std::io::Write;
use termion::{clear, cursor, event::Key, input::TermRead, raw::IntoRawMode};

use super::Command;

#[derive(Parser, Debug)]
#[clap(about = "Search remotes and your local library")]
pub struct Interactive {}

impl Command for Interactive {
    fn run(&self) -> Result<()> {
        let mut stdin = std::io::stdin().keys();
        let mut stdout = std::io::stdout().into_raw_mode()?;
        let (sender, receiver) = async_std::channel::unbounded();

        let local_rec = receiver.clone();

        let mut state: State = State::Searching;
        let mut data = StateData::new();

        write!(stdout, "{}{} ", cursor::Goto(1, 1), clear::All)?;

        task::spawn(async move {
            while let Ok(search_string) = receiver.recv().await {
                let query = Query::builder().terms(vec![search_string]).build();
                let hits = remotes::dblp::DBLP::fetch(query).await;
                if let Ok(hits) = hits {
                    for (i, hit) in hits.into_iter().enumerate().take(5) {
                        write!(
                            std::io::stdout(),
                            "{hide}{goto}{clear}{}",
                            hit,
                            hide = cursor::Hide,
                            goto = cursor::Goto(1, i as u16 + 4),
                            clear = clear::CurrentLine
                        )
                        .ok();
                    }
                    std::io::stdout().flush().ok();
                }
            }
        });

        loop {
            let k = stdin.next();

            if let Some(Ok(key)) = k {
                let action = handle_key(key, &state, &mut data)?;

                write!(
                    std::io::stdout(),
                    "{hide}{goto}{clear}{}",
                    data.search_term,
                    hide = cursor::Hide,
                    goto = cursor::Goto(1, 2),
                    clear = clear::CurrentLine
                )
                .ok();
                std::io::stdout().flush().ok();

                update_state(&action, &mut state);
                match action {
                    Action::UpdateSearch => {
                        while let Ok(_) = local_rec.try_recv() {}
                        sender.try_send(data.search_term.clone())?
                    }
                    _ => break,
                }
            }
        }

        write!(
            std::io::stdout(),
            "{}{}{}",
            cursor::Goto(1, 1),
            cursor::Show,
            clear::All
        )
        .ok();

        Ok(())
    }
}

fn handle_key(key: Key, state: &State, data: &mut StateData) -> Result<Action> {
    match (key, state) {
        (Key::Esc | Key::Ctrl('c'), _) => Ok(Action::Quit),
        (Key::Char(c), State::Searching) => {
            data.search_term.push(c);
            Ok(Action::UpdateSearch)
        }
        (Key::Backspace, State::Searching) => {
            data.search_term.pop();
            Ok(Action::UpdateSearch)
        }
        (_, _) => Ok(Action::Quit),
    }
}

fn update_state(action: &Action, state: &mut State) {
    match action {
        Action::ToScroll => *state = State::Scrolling,
        Action::ToSearch => *state = State::Searching,
        _ => {}
    }
}

struct StateData {
    search_term: String,
}

impl StateData {
    fn new() -> Self {
        Self {
            search_term: String::new(),
        }
    }
}

enum State {
    Searching,
    Scrolling,
}

enum Action {
    UpdateSearch,
    ToSearch,
    ToScroll,

    Quit,
}
