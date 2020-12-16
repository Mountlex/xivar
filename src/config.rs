use std::path::PathBuf;

use anyhow::{bail, Result};

use std::env;

pub fn xivar_data_dir() -> Result<PathBuf> {
    let data_dir = match env::var_os("XIVAR_DATA_DIR") {
        Some(data_osstr) => PathBuf::from(data_osstr),
        None => match dirs_next::data_local_dir() {
            Some(mut data_dir) => {
                data_dir.push("xivar");
                data_dir
            }
            None => bail!("could not find database directory, please set XIVAR_DATA_DIR manually"),
        },
    };

    Ok(data_dir)
}

pub fn xivar_document_dir() -> Result<PathBuf> {
    let config_file = match dirs_next::config_dir() {
        Some(mut config_dir) => {
            config_dir.push("xivar");
            config_dir.push("xivar.toml");
            config_dir
        }
        None => bail!("could not find database directory, please set XIVAR_DOCUMENT_DIR manually"),
    };

    let mut settings = config::Config::default();
    if config_file.exists() {
        settings.merge(config::File::from(config_file))?;
    }

    settings.merge(config::Environment::with_prefix("XIVAR"))?;
    settings.set(
        "document_dir",
        dirs_next::document_dir()
            .unwrap()
            .as_path()
            .as_os_str()
            .to_str()
            .unwrap()
            .to_owned(),
    )?;

    let path = settings.get::<String>("document_dir")?;
    Ok(PathBuf::from(path))
}
