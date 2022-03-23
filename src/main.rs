mod cli;
//mod config;
mod identifier;
mod library;
mod paper;
mod query;
mod remotes;

use config::Config;
pub use identifier::*;
use once_cell::sync::Lazy;
pub use paper::*;
pub use query::Query;

use clap::Parser;
use cli::Cli;

use std::path::PathBuf;

use anyhow::{bail, Result};

pub fn xivar_data_dir() -> PathBuf {
    let path = CONFIG.get::<String>("data_dir").unwrap();
    PathBuf::from(path)
}

pub fn xivar_document_dir() -> PathBuf {
    let path = CONFIG.get::<String>("document_dir").unwrap();
    PathBuf::from(path)
}

fn load_config() -> Result<Config> {
    let config_file = match dirs_next::config_dir() {
        Some(mut config_dir) => {
            config_dir.push("xivar");
            config_dir.push("xivar.toml");
            config_dir
        }
        None => bail!("Could not find or create config file!"),
    };

    if !config_file.exists() {
        std::fs::create_dir_all(config_file.parent().unwrap()).unwrap();
        std::fs::File::create(&config_file)?;
    }

    let data_dir = match dirs_next::data_local_dir() {
        Some(mut data_dir) => {
            data_dir.push("xivar");
            data_dir
        }
        None => bail!("Could not find or create data dir!"),
    };

    let settings = config::Config::builder()
        .add_source(config::File::from(config_file))
        .set_default("document_dir", "")?
        .set_default(
            "data_dir",
            data_dir.as_os_str().to_str().unwrap().to_owned(),
        )?
        .build()?;

    Ok(settings)
}

pub(crate) static CONFIG: Lazy<Config> = Lazy::new(|| load_config().unwrap());

#[tokio::main]
async fn main() {
    if let Err(err) = set_up_logging() {
        println!("{}", err);
    }

    let app = Cli::parse();
    if let Err(err) = app.run().await {
        println!("{}", err);
    }
}

fn set_up_logging() -> Result<(), fern::InitError> {
    std::fs::create_dir_all("logs")?;
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{date}][{level}] {message}",
                date = chrono::Local::now().format("%H:%M:%S"),
                level = record.level(),
                message = message
            ));
        })
        .level(log::LevelFilter::Info)
        .chain(fern::log_file(format!(
            "logs/{}.log",
            chrono::Local::now().format("%d%m%Y-%H%M")
        ))?)
        .apply()?;

    log::info!("Logger set up!");

    Ok(())
}
