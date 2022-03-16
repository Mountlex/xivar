use std::path::PathBuf;

use anyhow::{anyhow, Result};

use reqwest::header::USER_AGENT;
use tokio::io::AsyncWriteExt;

use crate::library::LocalPaper;
use crate::{PaperInfo, PaperUrl};

pub async fn async_download_and_save(
    metadata: PaperInfo,
    download_url: PaperUrl,
    output: Option<&PathBuf>,
) -> Result<LocalPaper> {
    let dest = if let Some(output) = output {
        if output.is_file() {
            output.to_owned().with_extension("pdf")
        } else {
            output.join(metadata.default_filename())
        }
    } else {
        crate::xivar_document_dir().join(metadata.default_filename())
    }
    .with_extension("pdf");

    download_pdf(&download_url.raw(), &dest).await?;

    Ok(LocalPaper {
        metadata,
        location: dest,
        ees: vec![download_url],
    })
}

pub async fn download_pdf(url: &str, out_path: &PathBuf) -> Result<()> {
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
