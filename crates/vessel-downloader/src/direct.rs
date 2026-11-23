//! Resumable HTTP direct-file downloader (`MP4`, `MKV`, `WebM`) using `Range` requests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::header::{CONTENT_RANGE, CONTENT_TYPE, RANGE};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;
use url::Url;
use vessel_domain::{
    DownloadProgressSnapshot, DownloadStatus, EngineEvent, EventSink, VesselError, VesselResult,
};
use vessel_storage::{SqliteStorage, StoredDownload};

use crate::{apply_headers, bundle_subtitles};

const PROGRESS_FLUSH_BYTES: u64 = 256 * 1024;
const PROGRESS_FLUSH_INTERVAL: Duration = Duration::from_millis(250);

fn infer_media_extension(url: &str, content_type: Option<&str>) -> &'static str {
    let path_lower = Url::parse(url)
        .map(|u| u.path().to_ascii_lowercase())
        .unwrap_or_else(|_| url.to_ascii_lowercase());
    if path_lower.ends_with(".mkv") {
        return "mkv";
    }
    if path_lower.ends_with(".webm") {
        return "webm";
    }
    if path_lower.ends_with(".ts") {
        return "ts";
    }
    if path_lower.ends_with(".mp4") {
        return "mp4";
    }

    if let Some(ct) = content_type.map(str::to_ascii_lowercase) {
        if ct.contains("matroska") {
            return "mkv";
        }
        if ct.contains("webm") {
            return "webm";
        }
        if ct.contains("mp2t") {
            return "ts";
        }
    }
    "mp4"
}

fn parse_content_range_total(header_val: &str) -> Option<u64> {
    let (_, total_str) = header_val.trim().rsplit_once('/')?;
    if total_str == "*" {
        None
    } else {
        total_str.trim().parse::<u64>().ok()
    }
}

fn build_request(
    client: &reqwest::Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    existing_bytes: u64,
) -> VesselResult<reqwest::RequestBuilder> {
    let parsed = Url::parse(url.trim())
        .map_err(|err| VesselError::InvalidInput(format!("invalid download URL: {err}")))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(VesselError::InvalidInput(format!(
            "unsupported download URL scheme '{}'",
            parsed.scheme()
        )));
    }
    let mut req = apply_headers(client.get(parsed), headers);
    if existing_bytes > 0 {
        req = req.header(RANGE, format!("bytes={existing_bytes}-"));
    }
    Ok(req)
}

pub(crate) async fn run_direct_download(
    client: &reqwest::Client,
    storage: &Arc<SqliteStorage>,
    event_sink: &Arc<dyn EventSink>,
    record: &StoredDownload,
    cancel: &CancellationToken,
) -> VesselResult<PathBuf> {
    let local_dir = PathBuf::from(&record.local_dir);
    fs::create_dir_all(&local_dir)
        .await
        .map_err(|err| VesselError::Storage(format!("create download directory: {err}")))?;

    let part_path = local_dir.join("media.part");
    let existing_bytes = match fs::metadata(&part_path).await {
        Ok(meta) => meta.len(),
        Err(_) => 0,
    };

    let req = build_request(client, &record.source_url, &record.headers, existing_bytes)?;
    let mut resp = tokio::select! {
        _ = cancel.cancelled() => return Err(VesselError::Cancelled),
        res = req.send() => res.map_err(|err| VesselError::Network(format!("direct download failed: {err}")))?,
    };

    let status = resp.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && existing_bytes > 0 {
        // Already downloaded completely in .part file or server length shrank; verify from 0 if needed.
        let final_name = format!("media.{}", infer_media_extension(&record.source_url, None));
        let final_path = local_dir.join(final_name);
        fs::rename(&part_path, &final_path)
            .await
            .map_err(|err| VesselError::Storage(format!("finalize downloaded file: {err}")))?;
        return finalize_direct_completion(
            client,
            storage,
            event_sink,
            record,
            existing_bytes,
            &final_path,
        )
        .await;
    }

    if !status.is_success() {
        return Err(VesselError::Network(format!(
            "direct download returned HTTP {status}"
        )));
    }

    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let ext = infer_media_extension(&record.source_url, content_type.as_deref());
    let final_path = local_dir.join(format!("media.{ext}"));

    let (mut downloaded, total_bytes, append_mode) =
        if existing_bytes > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT {
            let total = resp
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_content_range_total)
                .or_else(|| resp.content_length().map(|len| existing_bytes + len));
            (existing_bytes, total, true)
        } else {
            (0u64, resp.content_length(), false)
        };

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append_mode)
        .truncate(!append_mode)
        .open(&part_path)
        .await
        .map_err(|err| VesselError::Storage(format!("open partial file: {err}")))?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);

    storage
        .update_download_progress(
            &record.id,
            DownloadStatus::Downloading,
            downloaded,
            total_bytes,
            0,
            1,
            None,
            None,
        )
        .await?;
    event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
        download_id: record.id.clone(),
        status: DownloadStatus::Downloading,
        bytes_downloaded: downloaded,
        total_bytes,
        segments_done: 0,
        segments_total: 1,
    }));

    let mut bytes_since_flush = 0u64;
    let mut last_emit = Instant::now();

    loop {
        let next_chunk = tokio::select! {
            _ = cancel.cancelled() => {
                let _ = writer.flush().await;
                storage
                    .update_download_progress(
                        &record.id,
                        DownloadStatus::Paused,
                        downloaded,
                        total_bytes,
                        0,
                        1,
                        None,
                        None,
                    )
                    .await?;
                event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
                    download_id: record.id.clone(),
                    status: DownloadStatus::Paused,
                    bytes_downloaded: downloaded,
                    total_bytes,
                    segments_done: 0,
                    segments_total: 1,
                }));
                return Err(VesselError::Cancelled);
            }
            res = resp.chunk() => {
                res.map_err(|err| VesselError::Network(format!("stream download chunk: {err}")))?
            }
        };

        let Some(chunk) = next_chunk else {
            break;
        };

        writer
            .write_all(&chunk)
            .await
            .map_err(|err| VesselError::Storage(format!("write download chunk: {err}")))?;
        let chunk_len = u64::try_from(chunk.len()).unwrap_or(0);
        downloaded = downloaded.saturating_add(chunk_len);
        bytes_since_flush = bytes_since_flush.saturating_add(chunk_len);

        if bytes_since_flush >= PROGRESS_FLUSH_BYTES
            || last_emit.elapsed() >= PROGRESS_FLUSH_INTERVAL
        {
            bytes_since_flush = 0;
            last_emit = Instant::now();
            storage
                .update_download_progress(
                    &record.id,
                    DownloadStatus::Downloading,
                    downloaded,
                    total_bytes,
                    0,
                    1,
                    None,
                    None,
                )
                .await?;
            event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
                download_id: record.id.clone(),
                status: DownloadStatus::Downloading,
                bytes_downloaded: downloaded,
                total_bytes,
                segments_done: 0,
                segments_total: 1,
            }));
        }
    }

    writer
        .flush()
        .await
        .map_err(|err| VesselError::Storage(format!("flush download buffer: {err}")))?;
    drop(writer);

    fs::rename(&part_path, &final_path)
        .await
        .map_err(|err| VesselError::Storage(format!("rename completed file: {err}")))?;

    finalize_direct_completion(client, storage, event_sink, record, downloaded, &final_path).await
}

async fn finalize_direct_completion(
    client: &reqwest::Client,
    storage: &Arc<SqliteStorage>,
    event_sink: &Arc<dyn EventSink>,
    record: &StoredDownload,
    downloaded: u64,
    final_path: &Path,
) -> VesselResult<PathBuf> {
    let local_dir = PathBuf::from(&record.local_dir);
    bundle_subtitles(client, &record.subtitles, &record.headers, &local_dir).await?;

    let final_str = final_path.to_string_lossy().into_owned();
    storage
        .update_download_progress(
            &record.id,
            DownloadStatus::Completed,
            downloaded,
            Some(downloaded),
            1,
            1,
            Some(&final_str),
            None,
        )
        .await?;
    event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
        download_id: record.id.clone(),
        status: DownloadStatus::Completed,
        bytes_downloaded: downloaded,
        total_bytes: Some(downloaded),
        segments_done: 1,
        segments_total: 1,
    }));

    Ok(final_path.to_path_buf())
}
