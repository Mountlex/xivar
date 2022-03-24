use std::path::PathBuf;

use anyhow::{bail, Result};

pub struct Config {
    pub data_dir: PathBuf,
    pub paper_dir: PathBuf,
}

pub fn load_config() -> Result<Config> {
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

    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).unwrap();
    }

    let settings = config::Config::builder()
        .add_source(config::File::from(config_file))
        .set_default("document_dir", "")?
        .set_default("data_dir", data_dir.as_os_str().to_str())?
        .build()?;

    Ok(Config {
        data_dir: settings.get::<PathBuf>("data_dir").unwrap(),
        paper_dir: settings.get::<PathBuf>("document_dir").unwrap(),
    })
}
