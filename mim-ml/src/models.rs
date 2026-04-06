use crate::error::{MlError, Result};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tracing::info;

const SCRFD_2_5G_URL: &str =
    "https://huggingface.co/DIAMONIK7777/antelopev2/resolve/main/scrfd_10g_bnkps.onnx";
const ARCFACE_URL: &str =
    "https://huggingface.co/public-data/insightface/resolve/main/models/buffalo_l/w600k_r50.onnx";

const SCRFD_FILENAME: &str = "scrfd_10g_bnkps.onnx";
const ARCFACE_FILENAME: &str = "w600k_r50.onnx";

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub filename: String,
    pub downloaded: u64,
    pub total: u64,
}

pub struct ModelManager {
    models_dir: PathBuf,
    progress_tx: Option<Arc<watch::Sender<Option<DownloadProgress>>>>,
}

impl ModelManager {
    pub fn new(models_dir: PathBuf) -> Self {
        Self { models_dir, progress_tx: None }
    }

    pub fn with_progress(mut self, tx: watch::Sender<Option<DownloadProgress>>) -> Self {
        self.progress_tx = Some(Arc::new(tx));
        self
    }

    pub fn scrfd_path(&self) -> PathBuf {
        self.models_dir.join(SCRFD_FILENAME)
    }

    pub fn arcface_path(&self) -> PathBuf {
        self.models_dir.join(ARCFACE_FILENAME)
    }

    pub async fn ensure_scrfd(&self) -> Result<PathBuf> {
        self.ensure_model(SCRFD_FILENAME, SCRFD_2_5G_URL).await
    }

    pub async fn ensure_arcface(&self) -> Result<PathBuf> {
        self.ensure_model(ARCFACE_FILENAME, ARCFACE_URL).await
    }

    pub async fn ensure_model(&self, filename: &str, url: &str) -> Result<PathBuf> {
        let path = self.models_dir.join(filename);
        if path.exists() {
            info!("Model already downloaded: {}", filename);
            return Ok(path);
        }

        info!("Downloading model: {} from {}", filename, url);
        self.download(url, &path, filename).await?;
        Ok(path)
    }

    async fn download(&self, url: &str, dest: &Path, filename: &str) -> Result<()> {
        std::fs::create_dir_all(&self.models_dir)
            .map_err(|e| MlError::DownloadFailed(format!("create dir: {e}")))?;

        let tmp_path = dest.with_extension("tmp");

        // Check for existing partial download to resume
        let existing_size = tokio::fs::metadata(&tmp_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let client = reqwest::Client::new();
        let mut request = client.get(url);

        // Request range if we have a partial file
        if existing_size > 0 {
            info!("Resuming download from {} bytes", existing_size);
            request = request.header("Range", format!("bytes={}-", existing_size));
        }

        let response = request
            .send()
            .await
            .map_err(|e| MlError::DownloadFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(MlError::DownloadFailed(format!("HTTP {}", status)));
        }

        let is_resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
        let total_size = if is_resumed {
            // Content-Range header gives total: "bytes 1000-9999/10000"
            response
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.rsplit('/').next())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(existing_size + response.content_length().unwrap_or(0))
        } else {
            response.content_length().unwrap_or(0)
        };

        info!(
            "Download {}: {:.1} MB total{}",
            filename,
            total_size as f64 / 1_048_576.0,
            if is_resumed { format!(", resuming from {:.1} MB", existing_size as f64 / 1_048_576.0) } else { String::new() }
        );

        // Open file: append if resuming, create if fresh
        let mut file = if is_resumed {
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&tmp_path)
                .await
                .map_err(|e| MlError::DownloadFailed(format!("open for resume: {e}")))?
        } else {
            tokio::fs::File::create(&tmp_path)
                .await
                .map_err(|e| MlError::DownloadFailed(format!("create file: {e}")))?
        };

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = if is_resumed { existing_size } else { 0 };
        let mut last_logged_pct = 0u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| MlError::DownloadFailed(e.to_string()))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| MlError::DownloadFailed(format!("write: {e}")))?;
            downloaded += chunk.len() as u64;

            // Emit progress
            if let Some(tx) = &self.progress_tx {
                let _ = tx.send(Some(DownloadProgress {
                    filename: filename.to_string(),
                    downloaded,
                    total: total_size,
                }));
            }

            if total_size > 0 {
                let pct = (downloaded * 100) / total_size;
                if pct >= last_logged_pct + 10 {
                    info!("Download progress: {}%", pct);
                    last_logged_pct = pct;
                }
            }
        }

        file.flush()
            .await
            .map_err(|e| MlError::DownloadFailed(format!("flush: {e}")))?;
        drop(file);

        std::fs::rename(&tmp_path, dest).map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path);
            MlError::DownloadFailed(format!("rename: {e}"))
        })?;

        // Clear progress
        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(None);
        }

        info!("Model download complete: {}", dest.display());
        Ok(())
    }
}
