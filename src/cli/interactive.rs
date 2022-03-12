use crate::{
    remotes::{self, Paper, PaperHit, Remote},
    Query,
};
use anyhow::Result;

use clap::Parser;
use std::{fmt::Display, io::Write};
use termion::{clear, color, cursor, event::Key, input::TermRead, raw::IntoRawMode};

use super::Command;

#[derive(Parser, Debug)]
#[clap(about = "Search remotes and your local library")]
pub struct Interactive {}

impl Command for Interactive {
    fn run(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct FetchResult {
    used_term: String,
    hits: Vec<PaperHit>,
}

async fn fetch(search_string: String) -> Result<FetchResult> {
    if search_string.is_empty() {
        return Ok(FetchResult {
            used_term: search_string,
            hits: vec![],
        });
    }
    log::warn!("fetching {}!", search_string);
    let query = Query::builder()
        .terms(vec![search_string.to_string()])
        .build();
    let hits = remotes::dblp::DBLP::fetch(query).await?;
    Ok(FetchResult {
        used_term: search_string,
        hits,
    })
}

pub async fn interactive() -> Result<()> {
    let mut stdout = std::io::stdout().into_raw_mode()?;
    write!(stdout, "{}{} ", cursor::Goto(1, 1), clear::All)?;

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel(32);

    let (query_tx, mut query_rx) = tokio::sync::mpsc::channel::<String>(1);
    let (fetch_tx, mut fetch_rx) = tokio::sync::mpsc::channel::<FetchResult>(1);

    let mut data = StateData::new();

    tokio::task::spawn(async move {
        while let Some(query) = (&mut query_rx).recv().await {
            if let Ok(fetch_res) = fetch(query).await {
                log::warn!("fetched hits! sending..");
                (&fetch_tx).send(fetch_res).await.unwrap();
            }
        }
    });

    tokio::task::spawn_blocking(move || {
        while let Some(key) = std::io::stdin().keys().next() {
            if let Ok(key) = key {
                stdin_tx.blocking_send(key).unwrap();
            }
        }
    });

    loop {
        tokio::select! {
            key = (&mut stdin_rx).recv() => {
                if let Some(key) = key {
                    log::warn!("key pressed {:?}", key);

                    if let Some(action) = handle_key(key, &mut data) {
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
                        match action {
                            Action::UpdateSearch => {
                                if !data.search_term.is_empty() {
                                    info(&String::from("Searching..."));
                                    if query_tx.capacity() > 0 {
                                        query_tx.try_send(data.search_term.to_string()).unwrap();
                                    }
                                }
                            }
                            Action::UpdateSelection => {
                                 log::warn!("Update selection");
                                 print_results(&data.hits, &data.search_term, data.selected)?;
                             }
                            Action::Quit => break,
                            _ => {}
                        }
                    }

                }
            },
            fetch_res = (&mut fetch_rx).recv() => {
                if let Some(fetch_res) = fetch_res {
                    log::warn!("received hits! printing...");
                    print_results(&fetch_res.hits, &data.search_term, None)?;
                    if fetch_res.used_term != data.search_term {
                        query_tx.try_send(data.search_term.to_string()).unwrap();
                    }
                    (&mut data).hits = fetch_res.hits;

                }
                // REDO if search term changed
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
    let current_state = data.state.clone();
    match (key, current_state) {
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
