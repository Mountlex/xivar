use crate::{
    remotes::{self, PaperHit, Remote},
    Query,
};
use anyhow::Result;
use async_std::{
    channel::{Receiver, Sender},
    task,
};
use log::warn;

use std::sync::{Arc, Mutex, RwLock};

use clap::Parser;
use std::{fmt::Display, io::Write};
use termion::{clear, color, cursor, event::Key, input::TermRead, raw::IntoRawMode};

use super::Command;

#[derive(Parser, Debug)]
#[clap(about = "Search remotes and your local library")]
pub struct Interactive {}

impl Command for Interactive {
    fn run(&self) -> Result<()> {
        let mut stdin = std::io::stdin().keys();
        let mut stdout = std::io::stdout().into_raw_mode()?;
        let (command_sender, command_receiver): (Sender<String>, Receiver<String>) =
            async_std::channel::unbounded();

        let (hit_sender, hit_receiver): (Sender<Vec<PaperHit>>, Receiver<Vec<PaperHit>>) =
            async_std::channel::unbounded();

        let mut data = StateData::new();

        write!(stdout, "{}{} ", cursor::Goto(1, 1), clear::All)?;

        let task_rec = command_receiver.clone();
        task::spawn(async move {
            while let Ok(search_string) = task_rec.recv().await {
                let query = Query::builder().terms(vec![search_string.clone()]).build();
                let hits = remotes::dblp::DBLP::fetch(query).await;
                if let Ok(hits) = hits {
                    print_results(&hits, &search_string, None);
                    hit_sender.send(hits).await;
                    // let n = hits.len();
                    // for (i, hit) in hits.into_iter().enumerate().take(5) {
                    //     write!(
                    //         std::io::stdout(),
                    //         "{hide}{goto}{clear}{}",
                    //         hit,
                    //         hide = cursor::Hide,
                    //         goto = cursor::Goto(1, i as u16 + 4),
                    //         clear = clear::CurrentLine
                    //     )
                    //     .ok();
                    // }

                    // info(&format!("Found {} results!", n));

                    // std::io::stdout().flush().ok();
                }
            }
        });

        loop {
            let k = stdin.next();

            if let Some(Ok(key)) = k {
                while let Ok(hits) = hit_receiver.try_recv() {
                    log::warn!("Fetched {} hits", hits.len());
                    data.hits = hits;
                }
                log::warn!("Pressed key {:?}", key);
                let action = handle_key(key, &mut data);
                write!(
                    std::io::stdout(),
                    "{hide}{goto}{clear}Search: {}",
                    data.search_term,
                    hide = cursor::Hide,
                    goto = cursor::Goto(1, 1),
                    clear = clear::CurrentLine
                )
                .ok();
                std::io::stdout().flush().ok();

                log::warn!("Current state {:?}", data.state);

                if let Some(action) = action {
                    match action {
                        Action::UpdateSearch => {
                            while let Ok(_) = command_receiver.try_recv() {}
                            if !data.search_term.is_empty() {
                                info(&String::from("Searching..."));
                                command_sender.try_send(data.search_term.clone())?
                            } else {
                                clear_results()
                            }
                        }
                        Action::UpdateSelection => {
                            log::warn!("Update selection");
                            print_results(&data.hits, &data.search_term, data.selected);
                        }
                        Action::Quit => break,
                        _ => {}
                    }
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

fn print_results(results: &[PaperHit], search_term: &str, selected: Option<u16>) -> Result<()> {
    let (width, height) = termion::terminal_size().unwrap_or((80, 20));
    for (i, paper) in results.into_iter().enumerate().take(10) {
        if Some(i as u16) == selected {
            write!(
                std::io::stdout(),
                "{hide}{goto}{clear}{color}{}",
                paper,
                hide = cursor::Hide,
                goto = cursor::Goto(1, i as u16 + 4),
                clear = clear::CurrentLine,
                color = color::Bg(color::Red),
            )?;
        } else {
            write!(
                std::io::stdout(),
                "{hide}{goto}{clear}{}",
                paper,
                hide = cursor::Hide,
                goto = cursor::Goto(1, i as u16 + 4),
                clear = clear::CurrentLine,
            )?;
        }
    }
    std::io::stdout().flush()?;
    Ok(())
}

fn handle_key(key: Key, data: &mut StateData) -> Option<Action> {
    let state = data.state.clone();
    match (key, state) {
        (Key::Esc | Key::Ctrl('c'), _) => Some(Action::Quit),
        (Key::Char(c), State::Searching) => {
            data.search_term.push(c);
            Some(Action::UpdateSearch)
        }
        (Key::Backspace, State::Searching) => {
            data.search_term.pop();
            Some(Action::UpdateSearch)
        }
        (Key::Down, State::Searching) => {
            if data.hits.len() > 0 {
                data.selected = Some(0);
                data.state = State::Scrolling;
                Some(Action::UpdateSelection)
            } else {
                None
            }
        }
        (Key::Down, State::Scrolling) => {
            let i = data.selected.unwrap();
            if i < data.hits.len() as u16 - 1 {
                *data.selected.as_mut().unwrap() = i + 1;
            }
            Some(Action::UpdateSelection)
        }
        (Key::Up, State::Scrolling) => {
            let i = data.selected.unwrap();
            if i > 0 {
                *data.selected.as_mut().unwrap() = i - 1;
            } else {
                data.state = State::Searching;
                data.selected = None;
            }
            Some(Action::UpdateSelection)
        }
        (Key::Char('s'), State::Scrolling) => {
            data.state = State::Searching;
            data.selected = None;
            Some(Action::UpdateSelection)
        }
        (_, _) => Some(Action::Quit),
    }
}

fn update_state(action: &Action, state: &mut State) {
    match action {
        Action::ToScroll => *state = State::Scrolling,
        Action::ToSearch => *state = State::Searching,
        _ => {}
    }
}

#[derive(Clone)]
struct StateData {
    search_term: String,
    selected: Option<u16>,
    hits: Vec<PaperHit>,
    state: State,
}

impl StateData {
    fn new() -> Self {
        Self {
            search_term: String::new(),
            selected: None,
            hits: vec![],
            state: State::Searching,
        }
    }
}

#[derive(Debug, Clone)]
enum State {
    Searching,
    Displaying,
    Scrolling,
}

enum Action {
    UpdateSearch,
    ToSearch,
    UpdateSelection,
    ToScroll,
    Quit,
}

fn clear_results() {
    write!(
        std::io::stdout(),
        "{hide}{goto}{clear}",
        hide = cursor::Hide,
        goto = cursor::Goto(1, 2),
        clear = clear::AfterCursor
    )
    .ok();
    std::io::stdout().flush().ok();
}

fn info<I: Display>(item: &I) -> usize {
    let buf = format!("{}", item);
    write!(
        std::io::stdout(),
        "{hide}{goto}{clear}{}",
        buf,
        hide = cursor::Hide,
        goto = cursor::Goto(1, 2),
        clear = clear::CurrentLine
    )
    .ok();
    std::io::stdout().flush().ok();
    buf.len()
}
