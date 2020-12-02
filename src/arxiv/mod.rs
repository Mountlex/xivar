use std::io::Write;

use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub async fn download_pdf(url: &str, out_path: &PathBuf) -> Result<()> {
    let mut response = surf::get(&url).await.map_err(|err| anyhow!(err))?;
    let body = response.body_bytes().await.map_err(|err| anyhow!(err))?;
    let mut file = std::fs::File::create(out_path.with_extension("pdf"))?;
    file.write_all(&body)?;
    Ok(())
}
