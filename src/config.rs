use std::path::PathBuf;

use anyhow::{Result, bail};

use std::env;


pub fn xivar_data_dir() -> Result<PathBuf> {
    let data_dir = match env::var_os("XIVAR_DATA_DIR") {
        Some(data_osstr) => PathBuf::from(data_osstr),
        None => match dirs::data_local_dir() {
            Some(mut data_dir) => {
                data_dir.push("xivar");
                data_dir
            }
            None => bail!("could not find database directory, please set XIVAR_DATA_DIR manually"),
        },
    };

    Ok(data_dir)
}