//! Resumable HTTP direct-file downloader (`MP4`, `MKV`, `WebM`) using `Range` requests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use url::Url;
use vessel_domain::{
    DownloadProgressSnapshot, DownloadStatus, EngineEvent, EventSink, VesselError, VesselResult,
};
use vessel_storage::{SqliteStorage, StoredDownload};

use crate::{apply_headers, bundle_subtitles};

const PROGRESS_FLUSH_BYTES: u64 = 256 * 1024;
const PROGRESS_FLUSH_INTERVAL: Duration = Duration::from_millis(250);
const DIRECT_CHUNK_CONCURRENCY: usize = 4;
const MIN_MULTIPART_BYTES: u64 = 2 * 1024 * 1024;

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

    if existing_bytes == 0
        && let Some((total_len, content_type)) =
            probe_multipart_support(client, &record.source_url, &record.headers, cancel).await?
        && let Some(final_path) = try_multipart_direct_download(
            client,
            storage,
            event_sink,
            record,
            &local_dir,
            total_len,
            content_type.as_deref(),
            cancel,
        )
        .await?
    {
        return finalize_direct_completion(
            client,
            storage,
            event_sink,
            record,
            total_len,
            &final_path,
        )
        .await;
    }

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

async fn probe_multipart_support(
    client: &reqwest::Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    cancel: &CancellationToken,
) -> VesselResult<Option<(u64, Option<String>)>> {
    let parsed = Url::parse(url.trim())
        .map_err(|err| VesselError::InvalidInput(format!("invalid download URL: {err}")))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Ok(None);
    }
    let req = apply_headers(client.head(parsed), headers);
    let resp = match tokio::select! {
        _ = cancel.cancelled() => return Err(VesselError::Cancelled),
        res = req.send() => res,
    } {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    if !resp.status().is_success() {
        return Ok(None);
    }
    let accepts_ranges = resp
        .headers()
        .get(ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("bytes"));
    let total_len = resp
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0);
    if !accepts_ranges || total_len < MIN_MULTIPART_BYTES {
        return Ok(None);
    }
    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    Ok(Some((total_len, content_type)))
}

#[allow(clippy::too_many_arguments)]
async fn try_multipart_direct_download(
    client: &reqwest::Client,
    storage: &Arc<SqliteStorage>,
    event_sink: &Arc<dyn EventSink>,
    record: &StoredDownload,
    local_dir: &Path,
    total_len: u64,
    content_type: Option<&str>,
    cancel: &CancellationToken,
) -> VesselResult<Option<PathBuf>> {
    let ext = infer_media_extension(&record.source_url, content_type);
    let final_path = local_dir.join(format!("media.{ext}"));
    let parts_count = u64::try_from(DIRECT_CHUNK_CONCURRENCY).unwrap_or(4);
    let chunk_size = total_len.div_ceil(parts_count);

    storage
        .update_download_progress(
            &record.id,
            DownloadStatus::Downloading,
            0,
            Some(total_len),
            0,
            DIRECT_CHUNK_CONCURRENCY as u32,
            None,
            None,
        )
        .await?;
    event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
        download_id: record.id.clone(),
        status: DownloadStatus::Downloading,
        bytes_downloaded: 0,
        total_bytes: Some(total_len),
        segments_done: 0,
        segments_total: DIRECT_CHUNK_CONCURRENCY as u32,
    }));

    let downloaded_total = Arc::new(AtomicU64::new(0));
    let done_chunks = Arc::new(AtomicU64::new(0));
    let flush_lock = Arc::new(Mutex::new(Instant::now()));
    let mut chunk_paths = Vec::with_capacity(DIRECT_CHUNK_CONCURRENCY);
    let mut join_set: JoinSet<VesselResult<()>> = JoinSet::new();

    for idx in 0..DIRECT_CHUNK_CONCURRENCY {
        let start = (idx as u64) * chunk_size;
        if start >= total_len {
            break;
        }
        let end = ((idx as u64 + 1) * chunk_size - 1).min(total_len - 1);
        let chunk_path = local_dir.join(format!("chunk_{idx}.part"));
        chunk_paths.push(chunk_path.clone());

        let client = client.clone();
        let storage = Arc::clone(storage);
        let event_sink = Arc::clone(event_sink);
        let downloaded_total = Arc::clone(&downloaded_total);
        let done_chunks = Arc::clone(&done_chunks);
        let flush_lock = Arc::clone(&flush_lock);
        let headers = record.headers.clone();
        let url = record.source_url.clone();
        let download_id = record.id.clone();
        let cancel = cancel.clone();

        join_set.spawn(async move {
            let req = apply_headers(client.get(&url), &headers)
                .header(RANGE, format!("bytes={start}-{end}"));
            let mut resp = tokio::select! {
                _ = cancel.cancelled() => return Err(VesselError::Cancelled),
                res = req.send() => res.map_err(|err| VesselError::Network(format!("multipart chunk {idx} failed: {err}")))?,
            };
            if resp.status() != reqwest::StatusCode::PARTIAL_CONTENT {
                return Err(VesselError::InvalidInput(
                    "server ignored Range header during multipart download".into(),
                ));
            }
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&chunk_path)
                .await
                .map_err(|err| VesselError::Storage(format!("open chunk_{idx}: {err}")))?;
            let mut writer = BufWriter::with_capacity(64 * 1024, file);

            loop {
                let next_chunk = tokio::select! {
                    _ = cancel.cancelled() => return Err(VesselError::Cancelled),
                    res = resp.chunk() => res.map_err(|err| VesselError::Network(format!("read chunk_{idx}: {err}")))?,
                };
                let Some(bytes) = next_chunk else {
                    break;
                };
                writer
                    .write_all(&bytes)
                    .await
                    .map_err(|err| VesselError::Storage(format!("write chunk_{idx}: {err}")))?;
                let agg = downloaded_total
                    .fetch_add(u64::try_from(bytes.len()).unwrap_or(0), Ordering::AcqRel)
                    + u64::try_from(bytes.len()).unwrap_or(0);

                if let Ok(mut last) = flush_lock.try_lock()
                    && last.elapsed() >= PROGRESS_FLUSH_INTERVAL
                {
                    *last = Instant::now();
                    let segs_done = u32::try_from(done_chunks.load(Ordering::Acquire)).unwrap_or(0);
                    let _ = storage
                        .update_download_progress(
                            &download_id,
                            DownloadStatus::Downloading,
                            agg,
                            Some(total_len),
                            segs_done,
                            DIRECT_CHUNK_CONCURRENCY as u32,
                            None,
                            None,
                        )
                        .await;
                    event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
                        download_id: download_id.clone(),
                        status: DownloadStatus::Downloading,
                        bytes_downloaded: agg,
                        total_bytes: Some(total_len),
                        segments_done: segs_done,
                        segments_total: DIRECT_CHUNK_CONCURRENCY as u32,
                    }));
                }
            }
            writer
                .flush()
                .await
                .map_err(|err| VesselError::Storage(format!("flush chunk_{idx}: {err}")))?;
            done_chunks.fetch_add(1, Ordering::AcqRel);
            Ok(())
        });
    }

    let mut first_err: Option<VesselError> = None;
    loop {
        let next_res = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                join_set.abort_all();
                if first_err.is_none() {
                    first_err = Some(VesselError::Cancelled);
                }
                break;
            }
            maybe = join_set.join_next() => match maybe {
                Some(res) => res,
                None => break,
            }
        };
        match next_res {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                if first_err.is_none() {
                    first_err = Some(err);
                    cancel.cancel();
                }
            }
            Err(err) => {
                if err.is_cancelled() {
                    if first_err.is_none() {
                        first_err = Some(VesselError::Cancelled);
                    }
                } else if first_err.is_none() {
                    first_err = Some(VesselError::Storage(format!(
                        "multipart download worker panicked: {err}"
                    )));
                    cancel.cancel();
                }
            }
        }
    }

    if let Some(err) = first_err {
        for p in &chunk_paths {
            let _ = fs::remove_file(p).await;
        }
        if matches!(err, VesselError::InvalidInput(_)) {
            return Ok(None);
        }
        return Err(err);
    }

    let assembled = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&final_path)
        .await
        .map_err(|err| VesselError::Storage(format!("create assembled file: {err}")))?;
    let mut out = BufWriter::with_capacity(256 * 1024, assembled);
    for p in &chunk_paths {
        let mut src = fs::File::open(p)
            .await
            .map_err(|err| VesselError::Storage(format!("open chunk for merge: {err}")))?;
        tokio::io::copy(&mut src, &mut out)
            .await
            .map_err(|err| VesselError::Storage(format!("merge chunk: {err}")))?;
        let _ = fs::remove_file(p).await;
    }
    out.flush()
        .await
        .map_err(|err| VesselError::Storage(format!("flush merged file: {err}")))?;

    Ok(Some(final_path))
}
