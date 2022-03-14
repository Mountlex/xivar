mod state;

use crate::{
    cli::util::async_download_and_save,
    library::{lib_manager_fut, LibReq},
    remotes::{self, FetchResult, Remote},
    PaperInfo, PaperUrl, Query,
};
use anyhow::Result;

use std::io::Write;
use termion::{clear, cursor, input::TermRead, raw::IntoRawMode};
use tokio::sync::watch;

use self::state::StateData;

pub async fn interactive() -> Result<()> {
    let mut stdout = std::io::stdout().into_raw_mode()?;
    write!(stdout, "{}{} ", cursor::Goto(1, 1), clear::All)?;
    let mut data = StateData::new();
    data.write_to_terminal(&mut stdout)?;

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel(32);
    let (query_tx, query_rx) = tokio::sync::watch::channel::<String>(String::new());
    let (result_tx, mut result_rx) = tokio::sync::mpsc::unbounded_channel::<Result<FetchResult>>();

    let input_handle = tokio::task::spawn_blocking(move || {
        while let Some(key) = std::io::stdin().keys().next() {
            if let Ok(key) = key {
                if let Err(_) = stdin_tx.blocking_send(key) {
                    // TODO
                }
            }
        }
    });

    let arxiv_handle = tokio::task::spawn(fetch_manager(
        remotes::arxiv::Arxiv,
        query_rx.clone(),
        result_tx.clone(),
    ));

    let dblp_handle = tokio::task::spawn(fetch_manager(
        remotes::dblp::DBLP,
        query_rx.clone(),
        result_tx.clone(),
    ));

    let (local_tx, local_rx) = tokio::sync::mpsc::channel::<LibReq>(32);
    let local_handle = tokio::task::spawn(fetch_manager(
        remotes::local::LocalRemote::with_sender(local_tx.clone()),
        query_rx.clone(),
        result_tx.clone(),
    ));
    let lib_manager_handle = tokio::task::spawn(lib_manager_fut(local_rx));

    let mut remotes_fetched: usize = 0;
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
                    remotes_fetched += 1;
                    if let Ok(ok_res) = fetch_res {
                        data.merge_to_papers(ok_res.hits);
                    }
                    // TODO count whether all results arrived
                    if remotes_fetched == 3 {
                        data.to_idle()
                    }
                    data.write_to_terminal(&mut stdout)?;
                }
            }
        }
    }

    lib_manager_handle.abort();
    arxiv_handle.abort();
    dblp_handle.abort();
    local_handle.abort();
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
    std::process::Command::new("clear").status().unwrap();
    Ok(())
}

async fn fetch_manager<R: Remote + std::marker::Send + Clone>(
    remote: R,
    mut query_watch: watch::Receiver<String>,
    result_sender: tokio::sync::mpsc::UnboundedSender<Result<FetchResult>>,
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
            else => break
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
