use crate::{
    cli::util::async_download_and_save,
    remotes::{
        self,
        local::{get_local_hits, Library, LocalPaper},
        merge_to_papers, Paper, PaperHit, Remote, RemoteTag,
    },
    PaperInfo, PaperUrl, Query,
};
use anyhow::Result;

use async_trait::async_trait;
use clap::Parser;
use itertools::Itertools;
use std::{fmt::Display, io::Write};
use termion::{clear, color, cursor, event::Key, input::TermRead, raw::IntoRawMode};
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct LocalRemote {
    query_sender: tokio::sync::mpsc::Sender<LocalReq>,
}

#[async_trait]
impl Remote for LocalRemote {
    async fn fetch_from_remote(&self, query: Query) -> Result<Vec<PaperHit>> {
        let (res_sender, res_recv) = tokio::sync::oneshot::channel::<Vec<PaperHit>>();
        self.query_sender
            .send(LocalReq::Query {
                res_channel: res_sender,
                query,
            })
            .await?;
        res_recv.await.map_err(|err| anyhow::anyhow!(err))
    }
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

#[derive(Debug)]
enum LocalReq {
    Save {
        paper: LocalPaper,
    },
    Query {
        res_channel: tokio::sync::oneshot::Sender<Vec<PaperHit>>,
        query: Query,
    },
}

async fn lib_manager_fut(mut req_recv: tokio::sync::mpsc::Receiver<LocalReq>) -> Result<()> {
    let data_dir = crate::config::xivar_data_dir()?;
    let mut lib = Library::open(&data_dir)?;

    while let Some(req) = req_recv.recv().await {
        match req {
            LocalReq::Save { paper } => {
                lib.add(paper);
            }
            LocalReq::Query { res_channel, query } => {
                let results = get_local_hits(&lib, &query);
                res_channel.send(results).unwrap();
            }
        }
    }
    Ok(())
}

fn build_query(string: String) -> Query {
    Query::builder()
        .terms(
            string
                .split_whitespace()
                .map(|s| s.trim().to_lowercase())
                .collect(),
        )
        .build()
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

    let (local_tx, local_rx) = tokio::sync::mpsc::channel::<LocalReq>(32);
    tokio::task::spawn(fetch_manager_fut(
        LocalRemote {
            query_sender: local_tx.clone(),
        },
        query_rx.clone(),
        result_tx.clone(),
    ));
    let lib_manager_handle = tokio::task::spawn(lib_manager_fut(local_rx));

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
                            Action::Download(info, url) => {
                                let tx = local_tx.clone();
                                //let res_tx = result_tx.clone();
                                tokio::task::spawn(async move {
                                    let paper = async_download_and_save(info, url, None).await?;
                                    //res_tx.send(vec![PaperHit::Local(paper.clone())])?;
                                    log::warn!("Sending save request...");
                                    tx.send(LocalReq::Save { paper }).await?;
                                    log::warn!("Sended save request");
                                    Ok::<(), anyhow::Error>(())
                                });
                            },
                            Action::FetchToClip(url) => {
                                tokio::task::spawn(async move {
                                let response = reqwest::get(&url.raw()).await.map_err(|err| anyhow::anyhow!(err))?;
                                let body: String = response.text().await.map_err(|err| anyhow::anyhow!(err))?;
                                tokio::fs::write("/tmp/xivar.bib", body).await?;
                                open::that("/tmp/xivar.bib")?;
                                Ok::<(), anyhow::Error>(())
                            });
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

    lib_manager_handle.abort();
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
    let (_width, height) = termion::terminal_size().unwrap_or((80, 20));

    write!(
        writer,
        "{hide}{goto}{clear}Search: {}",
        data.search_term,
        hide = cursor::Hide,
        goto = cursor::Goto(1, 1),
        clear = clear::CurrentLine
    )
    .ok();

    match &data.state {
        State::Searching => info(&String::from("Searching...")),
        State::Scrolling(i) => {
            let selected: &Paper = &data.hits[*i as usize];
            let string: String = selected
                .0
                .iter()
                .enumerate()
                .map(|(i, hit)| format!("({}) {}", i + 1, hit.remote_tag()))
                .join("  ");
            info(&format!("Select remote: {}", string))
        }
        State::Idle => {
            if data.hits.len() > 0 {
                info(&format!("Found {} results!", data.hits.len()));
            } else {
                info(&String::from(""));
            }
        }
        State::SelectedHit { index: _, hit } => match hit {
            PaperHit::Local(paper) => {
                info(&format!("Select action: (1) open {:?}", paper.location))
            }
            PaperHit::Dblp(paper) => info(&format!(
                "Select action: (1) {:15}  (2) {:15}  (3) Show bib file",
                paper.ee.raw(),
                paper.url.raw()
            )),
            PaperHit::Arxiv(_) => info(&format!("Select action: (1) Download  (2) open online")),
        },
    }

    for (i, paper) in data.hits.iter().enumerate().take((height - 5) as usize) {
        if data.state == State::Scrolling(i as u16) {
            write!(
                writer,
                "{hide}{goto}{clear}{color}{}",
                paper,
                hide = cursor::Hide,
                goto = cursor::Goto(1, i as u16 + 4),
                clear = clear::CurrentLine,
                color = color::Bg(color::Blue),
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
        (Key::Ctrl('c'), _) => Some(Action::Quit),
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
                data.state = State::Scrolling(0);
                Some(Action::Reprint)
            } else {
                None
            }
        }
        (Key::Down, State::Scrolling(i)) => {
            if i < data.hits.len() as u16 - 1 {
                data.state = State::Scrolling(i + 1);
            }
            Some(Action::Reprint)
        }
        (Key::Up, State::Scrolling(i)) => {
            if i > 0 {
                data.state = State::Scrolling(i - 1);
            } else {
                data.state = State::Idle;
            }
            Some(Action::Reprint)
        }
        (Key::Char('s'), State::Scrolling(_)) => {
            data.state = State::Idle;
            Some(Action::Reprint)
        }
        (Key::Char('\n'), State::Scrolling(i)) => {
            let selected: &Paper = &data.hits[i as usize];
            let hit = selected.0.first().unwrap();
            match hit {
                PaperHit::Local(paper) => open::that(&paper.location).ok()?,
                PaperHit::Dblp(ref paper) => open::that(paper.ee.raw()).ok()?,
                PaperHit::Arxiv(ref paper) => open::that(paper.ee.raw()).ok()?,
            }
            None
        }
        (Key::Char(s), State::Scrolling(i)) => {
            if s.is_numeric() {
                let j = s.to_digit(10).unwrap();
                let selected: &Paper = &data.hits[i as usize];
                let selected_hit = &selected.0[(j - 1) as usize];
                data.state = State::SelectedHit {
                    index: i,
                    hit: selected_hit.clone(),
                };
                Some(Action::Reprint)
            } else {
                None
            }
        }
        (Key::Char(s), State::SelectedHit { index: _, hit }) => {
            match &hit {
                PaperHit::Local(paper) => open::that(&paper.location).ok()?,
                PaperHit::Dblp(paper) => {
                    if s == '1' {
                        open::that(paper.ee.raw()).ok()?
                    }
                    if s == '2' {
                        open::that(paper.url.raw()).ok()?
                    }
                    if s == '3' {
                        return Some(Action::FetchToClip(paper.bib_url()));
                    }
                }
                PaperHit::Arxiv(paper) => {
                    if s == '1' {
                        return Some(Action::Download(
                            paper.metadata().clone(),
                            paper.download_url(),
                        ));
                    }
                    if s == '2' {
                        open::that(paper.ee.raw()).ok()?
                    }
                }
            }
            Some(Action::Reprint)
        }
        (Key::Esc, State::SelectedHit { index: i, hit: _ }) => {
            data.state = State::Scrolling(i);
            Some(Action::Reprint)
        }
        (Key::Esc, State::Scrolling(_)) => {
            data.state = State::Searching;
            Some(Action::Reprint)
        }
        _ => None,
    }
}

#[derive(Clone)]
struct StateData {
    search_term: String,
    hits: Vec<Paper>,
    state: State,
}

impl StateData {
    fn new() -> Self {
        Self {
            search_term: String::new(),
            hits: vec![],
            state: State::Idle,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Idle,
    Searching,
    Scrolling(u16),
    SelectedHit { index: u16, hit: PaperHit },
}

enum Action {
    UpdateSearch,
    FetchToClip(PaperUrl),
    Download(PaperInfo, PaperUrl),
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
