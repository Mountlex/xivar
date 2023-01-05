mod state;

use crate::{
    library::{lib_manager_fut, LibReq, LoadingResult},
    remotes::{self, FetchResult, Remote},
    util::async_download_and_save,
    xiv_config::Config,
    PaperInfo, PaperUrl, Query,
};
use anyhow::Result;
use console::style;
use itertools::Itertools;

use std::{io::Write, path::PathBuf, time::Duration};
use termion::{clear, cursor, event::Key, input::TermRead, raw::IntoRawMode};
use tokio::sync::watch;

use self::state::StateData;

pub async fn interactive(config: Config) -> Result<()> {
    let mut stdout = std::io::stdout().into_raw_mode()?;
    write!(stdout, "{}{} ", cursor::Goto(1, 1), clear::All)?;
    let mut data = StateData::new();
    data.write_to_terminal(&mut stdout)?;

    let (width, height) = termion::terminal_size().unwrap_or((80, 20));
    log::info!("Terminal size ({}, {})", width, height);

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel(32);
    let (progress_tx, progress_rx) = tokio::sync::mpsc::channel::<ProgressRequest>(32);
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

    tokio::task::spawn(progress_manager(progress_rx, shutdown_tx.subscribe(), 50));

    tokio::task::spawn(fetch_manager(
        remotes::arxiv::Arxiv,
        query_rx.clone(),
        result_tx.clone(),
        progress_tx.clone(),
        shutdown_tx.subscribe(),
    ));

    tokio::task::spawn(fetch_manager(
        remotes::dblp::Dblp,
        query_rx.clone(),
        result_tx.clone(),
        progress_tx.clone(),
        shutdown_tx.subscribe(),
    ));

    let (local_tx, local_rx) = tokio::sync::mpsc::channel::<LibReq>(32);
    tokio::task::spawn(fetch_manager(
        remotes::local::LocalRemote::with_sender(local_tx.clone()),
        query_rx.clone(),
        result_tx.clone(),
        progress_tx.clone(),
        shutdown_tx.subscribe(),
    ));
    let (loading_tx, mut loading_rx) = tokio::sync::mpsc::channel::<LoadingResult>(1);
    tokio::task::spawn(lib_manager_fut(
        config.data_dir.clone(),
        local_rx,
        shutdown_tx.subscribe(),
        loading_tx,
    ));

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
                                tokio::task::spawn(download_paper(config.paper_dir.clone(),info, url, local_tx.clone(), progress_tx.clone()));
                            },
                            Action::FetchToClip(url) => {
                                tokio::task::spawn(async move {
                                    let response = reqwest::get(&url.raw()).await.map_err(|err| anyhow::anyhow!(err))?;
                                    let body: String = response.text().await.map_err(|err| anyhow::anyhow!(err))?;
                                    cli_clipboard::set_contents(body).unwrap();
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
                        data.set_idle_state()
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

#[derive(Debug)]
enum ProgressRequest {
    Start(String, u16),
    Finish(String),
}

async fn download_paper(
    paper_dir: PathBuf,
    info: PaperInfo,
    url: PaperUrl,
    local_tx: tokio::sync::mpsc::Sender<LibReq>,
    progress_tx: tokio::sync::mpsc::Sender<ProgressRequest>,
) -> Result<()> {
    let msg = style("Download").green().to_string();
    progress_tx
        .send(ProgressRequest::Start(msg.clone(), 3))
        .await
        .unwrap();
    log::info!("Starting to download paper at {:?}", url);
    let dest = paper_dir
        .join(info.default_filename())
        .with_extension("pdf");
    let paper = async_download_and_save(info, url, &dest).await?;
    log::info!("Finished downloading paper!");
    local_tx.send(LibReq::Save { paper }).await.unwrap();
    progress_tx
        .send(ProgressRequest::Finish(msg))
        .await
        .unwrap();
    open::that(&dest)?;
    Ok::<(), anyhow::Error>(())
}

async fn progress_manager(
    mut progress_recv: tokio::sync::mpsc::Receiver<ProgressRequest>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ms: u64,
) {
    let mut running: Vec<(String, u16)> = vec![];
    let tick_strings: Vec<String> = "⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈ "
        .chars()
        .map(|c| c.to_string())
        .collect();
    let mut state = 0;
    loop {
        tokio::time::sleep(Duration::from_millis(ms)).await;

        if running.is_empty() {
            tokio::select! {
                _ = shutdown_rx.recv() => break,
                Some(req) = progress_recv.recv() => {
                    match req {
                        ProgressRequest::Start(name, line) => {
                            if !running.contains(&(name.clone(), line)) {
                                running.push((name, line));
                            }
                        }
                        ProgressRequest::Finish(name) => running.retain(|(n,_)| *n != name),
                    }
                }
            }
        } else if shutdown_rx.try_recv().is_ok() {
            break;
        }

        while let Ok(req) = progress_recv.try_recv() {
            match req {
                ProgressRequest::Start(name, line) => {
                    if !running.contains(&(name.clone(), line)) {
                        running.push((name, line));
                    }
                }
                ProgressRequest::Finish(name) => running.retain(|(n, _)| *n != name),
            }
        }

        if !running.is_empty() {
            for (l, names) in &running.iter().group_by(|(_, l)| l) {
                let text = names
                    .into_iter()
                    .map(|(n, _)| format!("{} {}", style(&tick_strings[state]).bold(), n))
                    .join("  ");
                write!(
                    std::io::stdout(),
                    "{}{}{}{}",
                    cursor::Goto(2, *l),
                    cursor::Hide,
                    clear::CurrentLine,
                    text
                )
                .ok();
            }
        } else {
            write!(
                std::io::stdout(),
                "{}{}{}",
                cursor::Goto(2, 3),
                cursor::Hide,
                clear::CurrentLine,
            )
            .ok();
        }
        std::io::stdout().flush().ok();

        state += 1;
        if state == tick_strings.len() {
            state = 0;
        }
    }
}

async fn fetch_manager<R: Remote + std::marker::Send + Clone>(
    remote: R,
    mut query_watch: watch::Receiver<String>,
    result_sender: tokio::sync::mpsc::UnboundedSender<Result<FetchResult>>,
    progress_sender: tokio::sync::mpsc::Sender<ProgressRequest>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut to_query: Option<String> = None;
    loop {
        tokio::select! {
            Ok(()) = query_watch.changed() => {
                if query_watch.borrow().is_empty() {
                    progress_sender.send(ProgressRequest::Finish(remote.name())).await.unwrap();
                    to_query = None;
                } else {
                    to_query = Some(query_watch.borrow().to_string());
                    progress_sender.send(ProgressRequest::Start(remote.name(), 2)).await.unwrap();
                }
            }
            result = remote.fetch_from_remote(Query::from(to_query.unwrap_or_default()), 30), if to_query.is_some() => {
                to_query = None;
                progress_sender.send(ProgressRequest::Finish(remote.name())).await.unwrap();
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
