use crate::{
    remotes::{self, PaperHit, Remote},
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
    let mut data = StateData::new();
    print_results(&mut stdout, &data)?;

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel(32);
    let input_handle = tokio::task::spawn_blocking(move || {
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

                        match action {
                            Action::Update => {
                                print_results(&mut stdout, &data)?;
                             }
                            Action::Quit => {
                                log::warn!("Quitting!");
                                break;
                            },
                        }
                    }

                }
            },
            fetch_res = fetch(data.search_term.clone()), if data.state == State::Searching => {
                if let Ok(fetch_res) = fetch_res {
                    let num_hits = fetch_res.hits.len();
                    (&mut data).hits = fetch_res.hits;
                    print_results(&mut stdout, &data)?;
                    if fetch_res.used_term == data.search_term {
                        data.state = State::Idle;
                        info(&format!("Found {} results!", num_hits));
                    }
                }
            }
        }
    }

    input_handle.abort();
    write!(
        stdout,
        "{}{}{}",
        cursor::Goto(1, 1),
        cursor::Show,
        clear::All
    )
    .ok();
    stdout.flush()?;

    Ok(())
}

fn print_results<W: std::io::Write>(writer: &mut W, data: &StateData) -> Result<()> {
    write!(
        writer,
        "{hide}{goto}{clear}",
        hide = cursor::Hide,
        goto = cursor::Goto(1, 4),
        clear = clear::AfterCursor
    )
    .ok();
    //let (width, height) = termion::terminal_size().unwrap_or((80, 20));

    write!(
        writer,
        "{hide}{goto}{clear}Search: {}",
        data.search_term,
        hide = cursor::Hide,
        goto = cursor::Goto(1, 1),
        clear = clear::CurrentLine
    )
    .ok();

    match data.state {
        State::Searching => info(&String::from("Searching...")),
        State::Scrolling => info(&String::from("Scrolling...")),
        State::Idle => {
            if data.hits.len() > 0 {
                info(&format!("Found {} results!", data.hits.len()));
            } else {
                info(&String::from(""));
            }
        }
    }

    for (i, paper) in data.hits.iter().enumerate().take(10) {
        if Some(i as u16) == data.selected {
            write!(
                writer,
                "{hide}{goto}{clear}{color}{}",
                paper,
                hide = cursor::Hide,
                goto = cursor::Goto(1, i as u16 + 4),
                clear = clear::CurrentLine,
                color = color::Bg(color::Red),
            )?;
        } else {
            write!(
                writer,
                "{hide}{goto}{clear}{}",
                paper,
                hide = cursor::Hide,
                goto = cursor::Goto(1, i as u16 + 4),
                clear = clear::CurrentLine,
            )?;
        }
    }
    writer.flush()?;
    Ok(())
}

fn handle_key(key: Key, data: &mut StateData) -> Option<Action> {
    let current_state = data.state.clone();
    match (key, current_state) {
        (Key::Esc | Key::Ctrl('c'), _) => Some(Action::Quit),
        (Key::Char(c), State::Idle) => {
            data.search_term.push(c);
            data.state = State::Searching;
            Some(Action::Update)
        }
        (Key::Char(c), State::Searching) => {
            data.search_term.push(c);
            Some(Action::Update)
        }
        (Key::Backspace, State::Searching | State::Idle) => {
            data.search_term.pop();
            if data.search_term.is_empty() {
                data.state = State::Idle;
                data.hits.clear();
            } else {
                data.state = State::Searching;
            }
            Some(Action::Update)
        }
        (Key::Down, State::Idle) => {
            if data.hits.len() > 0 {
                data.selected = Some(0);
                data.state = State::Scrolling;
                Some(Action::Update)
            } else {
                None
            }
        }
        (Key::Down, State::Scrolling) => {
            let i = data.selected.unwrap();
            if i < data.hits.len() as u16 - 1 {
                *data.selected.as_mut().unwrap() = i + 1;
            }
            Some(Action::Update)
        }
        (Key::Up, State::Scrolling) => {
            let i = data.selected.unwrap();
            if i > 0 {
                *data.selected.as_mut().unwrap() = i - 1;
            } else {
                data.state = State::Idle;
                data.selected = None;
            }
            Some(Action::Update)
        }
        (Key::Char('s'), State::Scrolling) => {
            data.state = State::Idle;
            data.selected = None;
            Some(Action::Update)
        }
        _ => Some(Action::Quit),
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
            state: State::Idle,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Idle,
    Searching,
    Scrolling,
}

enum Action {
    Update,
    Quit,
}

fn info<I: Display>(item: &I) {
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
}
