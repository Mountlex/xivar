mod state;

use crate::{
    cli::util::async_download_and_save,
    library::{lib_manager_fut, LibReq, LoadingResult},
    remotes::{self, FetchResult, Remote},
    PaperInfo, PaperUrl, Query,
};
use anyhow::Result;

use std::{io::Write, time::Duration};
use termion::{clear, cursor, event::Key, input::TermRead, raw::IntoRawMode};
use tokio::sync::watch;

use self::state::StateData;

pub async fn interactive() -> Result<()> {
    let mut stdout = std::io::stdout().into_raw_mode()?;
    write!(stdout, "{}{} ", cursor::Goto(1, 1), clear::All)?;
    let mut data = StateData::new();
    data.write_to_terminal(&mut stdout)?;

    let (width, height) = termion::terminal_size().unwrap_or((80, 20));
    log::info!("Terminal size ({}, {})", width, height);

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel(32);
    let (query_tx, query_rx) = tokio::sync::watch::channel::<String>(String::new());
    let (result_tx, mut result_rx) = tokio::sync::mpsc::unbounded_channel::<Result<FetchResult>>();

    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);

    tokio::task::spawn_blocking(move || {
        while let Some(key) = std::io::stdin().keys().next() {
            if let Ok(key) = key {
                if let Err(_) = stdin_tx.blocking_send(key) {
                    // TODO
                }
                if Key::Ctrl('c') == key {
                    break;
                }
            }
        }
    });

    tokio::task::spawn(async move {
        let mut stdout = std::io::stdout().into_raw_mode().unwrap();
        let mut state = 0;
        loop {
            if state == 0 {
                state = 1;
                log::info!("write1");
                write!(
                    stdout,
                    "{}{}{}",
                    cursor::Goto(1, 3),
                    clear::CurrentLine,
                    ": "
                )
                .ok();
            } else {
                state = 0;
                log::info!("write2");

                write!(
                    stdout,
                    "{}{}{}{}",
                    cursor::Goto(1, 3),
                    cursor::Hide,
                    clear::CurrentLine,
                    " :"
                )
                .ok();
            }
            stdout.flush().unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    tokio::task::spawn(fetch_manager(
        remotes::arxiv::Arxiv,
        query_rx.clone(),
        result_tx.clone(),
        shutdown_tx.subscribe(),
    ));

    tokio::task::spawn(fetch_manager(
        remotes::dblp::DBLP,
        query_rx.clone(),
        result_tx.clone(),
        shutdown_tx.subscribe(),
    ));

    let (local_tx, local_rx) = tokio::sync::mpsc::channel::<LibReq>(32);
    tokio::task::spawn(fetch_manager(
        remotes::local::LocalRemote::with_sender(local_tx.clone()),
        query_rx.clone(),
        result_tx.clone(),
        shutdown_tx.subscribe(),
    ));
    let (loading_tx, mut loading_rx) = tokio::sync::mpsc::channel::<LoadingResult>(1);
    tokio::task::spawn(lib_manager_fut(
        local_rx,
        shutdown_tx.subscribe(),
        loading_tx,
    ));

    let mut state = 0;

    let mut remotes_fetched: usize = 0;
    let mut total_remotes: usize = 2;
    loop {
        tokio::select! {
            key = (&mut stdin_rx).recv() => {
                if let Some(key) = key {
                    log::info!("Pressed key {:?}", key);

                    if let Some(action) = data.state_transition(key) {
                        match action {
                            Action::UpdateSearch => {
                                remotes_fetched = 0;
                                query_tx.send(data.term().to_string())?;
                                data.write_to_terminal(&mut stdout)?;
                                data.clear_papers()
                            },
                            Action::Reprint => {
                                data.write_to_terminal(&mut stdout)?;

                            }
                            Action::Quit => {
                                log::info!("Quitting!");
                                shutdown_tx.send(())?;
                                break;
                            },
                            Action::Download(info, url) => {
                                let tx = local_tx.clone();
                                //let res_tx = result_tx.clone();
                                tokio::task::spawn(async move {
                                    log::info!("Starting to download paper at {:?}", url);
                                    let paper = async_download_and_save(info, url, None).await?;
                                    log::info!("Finished downloading paper!");
                                    tx.send(LibReq::Save { paper }).await?;
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
                    if let Ok(ok_res) = fetch_res {
                        if Query::from(data.term().to_string()) == ok_res.query {
                            remotes_fetched += 1;
                            data.merge_to_papers(ok_res.hits);
                        }
                    }
                    if remotes_fetched == total_remotes {
                        data.to_idle()
                    }
                    data.write_to_terminal(&mut stdout)?;
                }
            },
            load_res = loading_rx.recv() => {
                if let Some(LoadingResult::Success) = load_res {
                    total_remotes += 1
                }
            },
        }
    }

    // shutdown
    drop(result_tx);
    let _ = result_rx.recv().await;
    let _ = stdin_rx.recv().await;

    // reset terminal
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

async fn fetch_manager<R: Remote + std::marker::Send + Clone>(
    remote: R,
    mut query_watch: watch::Receiver<String>,
    result_sender: tokio::sync::mpsc::UnboundedSender<Result<FetchResult>>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
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
            result = remote.fetch_from_remote(Query::from(to_query.unwrap_or_default())), if to_query.is_some() => {
                to_query = None;
                    if result_sender.send(result).is_err() {
                        break
                    }
            }
            _ = shutdown_rx.recv() => break,
            else => break,
        }
    }
}

pub enum Action {
    UpdateSearch,
    FetchToClip(PaperUrl),
    Download(PaperInfo, PaperUrl),
    Reprint,
    Quit,
}
