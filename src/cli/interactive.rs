use crate::{
    remotes::{self, merge_papers, merge_to_papers, Paper, PaperHit, Remote, RemoteTag},
    Query,
};
use anyhow::Result;

use clap::Parser;
use itertools::Itertools;
use std::{fmt::Display, io::Write};
use termion::{clear, color, cursor, event::Key, input::TermRead, raw::IntoRawMode};
use tokio::sync::watch;

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
    hits: Vec<Paper>,
}

async fn fetch_manager_fut<R: Remote + std::marker::Send + Clone>(
    remote: R,
    mut query_watch: watch::Receiver<String>,
    result_sender: tokio::sync::mpsc::UnboundedSender<Vec<PaperHit>>,
) {
    let mut to_query: Option<String> = None;
    loop {
        tokio::select! {
            Ok(()) = query_watch.changed() => {
                if query_watch.borrow().is_empty() {
                    to_query = None;
                } else {
                    to_query = Some(query_watch.borrow().to_string())
                }
            }
            result = remote.fetch_from_remote(build_query(to_query.clone().unwrap_or_default())), if to_query.is_some() => {
                to_query = None;
                if let Ok(result) = result {
                    if result_sender.send(result).is_err() {
                        break
                    }
                }
            }
            else => break
        }
    }
}

fn build_query(string: String) -> Query {
    Query::builder().terms(vec![string]).build()
}

pub async fn interactive() -> Result<()> {
    let mut stdout = std::io::stdout().into_raw_mode()?;
    write!(stdout, "{}{} ", cursor::Goto(1, 1), clear::All)?;
    let mut data = StateData::new();
    print_results(&mut stdout, &data)?;

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel(32);

    let (query_tx, query_rx) = tokio::sync::watch::channel::<String>(String::new());
    let (result_tx, mut result_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<PaperHit>>();

    let input_handle = tokio::task::spawn_blocking(move || {
        while let Some(key) = std::io::stdin().keys().next() {
            if let Ok(key) = key {
                if let Err(_) = stdin_tx.blocking_send(key) {
                    // TODO
                }
            }
        }
    });

    tokio::task::spawn(fetch_manager_fut(
        remotes::arxiv::Arxiv,
        query_rx.clone(),
        result_tx.clone(),
    ));

    tokio::task::spawn(fetch_manager_fut(
        remotes::dblp::DBLP,
        query_rx.clone(),
        result_tx.clone(),
    ));

    tokio::task::spawn(fetch_manager_fut(
        remotes::local::Local::load().unwrap(),
        query_rx.clone(),
        result_tx.clone(),
    ));

    let mut remotes_fetched: usize = 0;
    loop {
        tokio::select! {
            key = (&mut stdin_rx).recv() => {
                if let Some(key) = key {
                    log::warn!("key pressed {:?}", key);

                    if let Some(action) = handle_key(key, &mut data) {
                        match action {
                            Action::UpdateSearch => {
                                remotes_fetched = 0;
                                query_tx.send(data.search_term.clone())?;
                                print_results(&mut stdout, &data)?;
                                data.hits.clear()
                            },
                            Action::Reprint => {
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
            fetch_res = result_rx.recv() => {
                if let Some(fetch_res) = fetch_res {
                    remotes_fetched += 1;
                    (&mut data).hits = merge_to_papers(data.hits.clone(), fetch_res.into_iter())?;
                    // TODO count whether all results arrived
                    if remotes_fetched == 3 {
                        data.state = State::Idle;
                    }
                    print_results(&mut stdout, &data)?;
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
    let (width, height) = termion::terminal_size().unwrap_or((80, 20));

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
        State::Scrolling => {
            let selected: &Paper = &data.hits[data.selected.unwrap() as usize];
            let string: String = selected
                .0
                .iter()
                .enumerate()
                .map(|(i, hit)| format!("({}) {}", i + 1, hit.remote_tag()))
                .join("  ");
            info(&string)
        }
        State::Idle => {
            if data.hits.len() > 0 {
                info(&format!("Found {} results!", data.hits.len()));
            } else {
                info(&String::from(""));
            }
        }
    }

    for (i, paper) in data.hits.iter().enumerate().take((height - 5) as usize) {
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
            Some(Action::UpdateSearch)
        }
        (Key::Char(c), State::Searching) => {
            data.search_term.push(c);
            Some(Action::UpdateSearch)
        }
        (Key::Backspace, State::Searching | State::Idle) => {
            data.search_term.pop();
            if data.search_term.is_empty() {
                data.state = State::Idle;
                data.hits.clear();
            } else {
                data.state = State::Searching;
            }
            Some(Action::UpdateSearch)
        }
        (Key::Down, State::Idle) => {
            if data.hits.len() > 0 {
                data.selected = Some(0);
                data.state = State::Scrolling;
                Some(Action::Reprint)
            } else {
                None
            }
        }
        (Key::Down, State::Scrolling) => {
            let i = data.selected.unwrap();
            if i < data.hits.len() as u16 - 1 {
                *data.selected.as_mut().unwrap() = i + 1;
            }
            Some(Action::Reprint)
        }
        (Key::Up, State::Scrolling) => {
            let i = data.selected.unwrap();
            if i > 0 {
                *data.selected.as_mut().unwrap() = i - 1;
            } else {
                data.state = State::Idle;
                data.selected = None;
            }
            Some(Action::Reprint)
        }
        (Key::Char('s'), State::Scrolling) => {
            data.state = State::Idle;
            data.selected = None;
            Some(Action::Reprint)
        }
        _ => None,
    }
}

#[derive(Clone)]
struct StateData {
    search_term: String,
    selected: Option<u16>,
    hits: Vec<Paper>,
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
    UpdateSearch,
    Reprint,
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
