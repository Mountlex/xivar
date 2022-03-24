use std::path::Path;

use anyhow::{anyhow, Result};

use reqwest::header::USER_AGENT;
use tokio::io::AsyncWriteExt;

use crate::library::LocalPaper;
use crate::{PaperInfo, PaperUrl};

pub async fn async_download_and_save(
    metadata: PaperInfo,
    download_url: PaperUrl,
    dest: &Path,
) -> Result<LocalPaper> {
    download_pdf(&download_url.raw(), &dest).await?;

    Ok(LocalPaper {
        metadata,
        location: dest.to_path_buf(),
        ees: vec![download_url],
    })
}

async fn download_pdf(url: &str, out_path: &Path) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(&*url)
        .header(USER_AGENT, "xivar")
        .send()
        .await
        .map_err(|err| anyhow!(err))?;
    let body = response.bytes().await.map_err(|err| anyhow!(err))?;
    let mut file = tokio::fs::File::create(out_path.with_extension("pdf")).await?;
    file.write_all(&body).await?;
    Ok(())
}
