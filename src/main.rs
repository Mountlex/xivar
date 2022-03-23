mod clean;
mod identifier;
mod interactive;
mod library;
mod paper;
mod query;
mod remotes;
mod util;
mod xiv_config;

use clean::Clean;
pub use identifier::*;
pub use paper::*;
pub use query::Query;

use clap::Parser;
use clap::Subcommand;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(err) = set_up_logging() {
        println!("{}", err);
    }

    let app = App::parse();
    let config = xiv_config::load_config()?;

    if let Err(err) = app.run(config).await {
        println!("{}", err);
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[clap(
    version = "0.4.0",
    author = "Alexander Lindermayr <alexander.lindermayr97@gmail.com>",
    about = "Manage your local scientific library!"
)]
pub struct App {
    #[clap(subcommand)]
    helper: Option<Helpers>,
}

impl App {
    pub async fn run(&self, config: xiv_config::Config) -> Result<()> {
        if let Some(helper) = &self.helper {
            helper.run(config)
        } else {
            interactive::interactive(config).await
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Helpers {
    Clean(Clean),
}

impl Helpers {
    fn run(&self, config: xiv_config::Config) -> Result<()> {
        match &self {
            Helpers::Clean(h) => h.run(config),
        }
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
