mod cli;
mod config;
mod identifier;
mod library;
mod paper;
mod query;
mod remotes;

pub use identifier::*;
pub use paper::*;
pub use query::Query;

use clap::Parser;
use cli::Cli;

// fn main() {
//     set_up_logging();
//     if let Err(error) = Cli::parse().run() {
//         println!("{}", error);
//     }
// }

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
