use anyhow::{bail, Context, Result};
use futures::StreamExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

use crate::api::TokenProvider;
use crate::limiter::ConcurrencyGate;

pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|".contains(c) || (c as u32) < 0x20 {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed.chars().take(180).collect()
    }
}

/// Best-effort filename guess from an attachment record.
pub fn attachment_filename(rec: &serde_json::Value, url: &str) -> String {
    for k in ["filename", "fileName", "name", "originalName", "title"] {
        if let Some(v) = rec.get(k).and_then(|v| v.as_str()) {
            if !v.trim().is_empty() {
                return sanitize_filename(v.trim());
            }
        }
    }
    if let Ok(u) = reqwest::Url::parse(url) {
        if let Some(last) = u.path_segments().and_then(|mut s| s.next_back()) {
            if !last.is_empty() {
                return sanitize_filename(last);
            }
        }
    }
    let ext = Path::new(url.split('?').next().unwrap_or(""))
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    format!("attachment{}", if ext.is_empty() { ".bin".into() } else { format!(".{ext}") })
}

/**
 * Streams attachment binaries to disk. Resumable via a `.part` file +
 * HTTP Range when the server supports it. GET-only like the API client.
 */
#[derive(Clone)]
pub struct Downloader {
    gate: Arc<ConcurrencyGate>,
    http: reqwest::Client,
    api_origin: String,
    get_token: TokenProvider,
}

pub enum DownloadOutcome {
    Done { bytes: u64 },
    Skipped { bytes: u64 },
}

impl Downloader {
    pub fn new(gate: Arc<ConcurrencyGate>, api_origin: String, get_token: TokenProvider) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("datafulton-exporter/0.1")
            .build()
            .expect("reqwest client");
        Self {
            gate,
            http,
            api_origin,
            get_token,
        }
    }

    pub fn using(&self, gate: Arc<ConcurrencyGate>) -> Self {
        Self {
            gate,
            http: self.http.clone(),
            api_origin: self.api_origin.clone(),
            get_token: self.get_token.clone(),
        }
    }

    /// Partial download size on disk (`.part` or finished file).
    pub fn bytes_on_disk(dest: &Path) -> u64 {
        let part = part_path(dest);
        if part.exists() {
            return std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        }
        if dest.exists() {
            return std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
        }
        0
    }

    /// Download `url` → `dest`. Keeps partial progress in `dest.part`.
    pub async fn download(&self, url: &str, dest: &Path) -> Result<DownloadOutcome> {
        let this = self.clone();
        let url = url.to_string();
        let dest = dest.to_path_buf();
        self.gate
            .run(|| async move { this.download_inner(&url, &dest).await })
            .await
    }

    async fn download_inner(&self, url: &str, dest: &Path) -> Result<DownloadOutcome> {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let part = part_path(dest);

        if dest.exists() {
            let bytes = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
            return Ok(DownloadOutcome::Skipped { bytes });
        }

        let resume_from = if part.exists() {
            std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        let mut req = self.http.get(url);
        // Only send the bearer token to Prime's own origin.
        if reqwest::Url::parse(url)
            .map(|u| u.origin().ascii_serialization() == self.api_origin)
            .unwrap_or(false)
        {
            if let Some(token) = (self.get_token)() {
                req = req.bearer_auth(token);
            }
        }
        if resume_from > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
        }

        let res = req.send().await.context("download request")?;

        if res.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && part.exists() {
            // Server says the range is unsatisfiable — the .part is likely whole.
            tokio::fs::rename(&part, dest).await?;
            let bytes = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
            return Ok(DownloadOutcome::Done { bytes });
        }
        if !(res.status().is_success() || res.status() == reqwest::StatusCode::PARTIAL_CONTENT) {
            bail!("Download failed: {}", res.status());
        }

        // If we asked for a range but got a 200, restart the file.
        let appending = res.status() == reqwest::StatusCode::PARTIAL_CONTENT && resume_from > 0;
        let expected_total = res
            .content_length()
            .map(|n| n + if appending { resume_from } else { 0 });

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(appending)
            .truncate(!appending)
            .open(&part)
            .await?;

        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("reading download stream")?;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        drop(file);

        let size = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        if let Some(total) = expected_total {
            if size < total {
                bail!("Truncated download: {size}/{total} bytes");
            }
        }

        tokio::fs::rename(&part, dest).await?;
        Ok(DownloadOutcome::Done { bytes: size })
    }
}

fn part_path(dest: &Path) -> PathBuf {
    let mut p = dest.as_os_str().to_os_string();
    p.push(".part");
    PathBuf::from(p)
}
