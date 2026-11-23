//! Pure-Rust offline download manager (`Range` direct resume + 4-worker AES-128 HLS + subtitles).

pub mod direct;
pub mod hls;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tokio::fs;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use vessel_domain::{
    DownloadProgressSnapshot, DownloadStatus, EngineEvent, EventSink, SubtitleFormat,
    SubtitleSource, SubtitleTrack, VesselError, VesselResult,
};
use vessel_storage::{DownloadKind, SqliteStorage, StoredDownload};

pub use hls::remux_local_hls_to_mp4;

/// Maximum simultaneous download jobs (`PLAN.md` M5.3).
pub const MAX_CONCURRENT_DOWNLOADS: usize = 2;

static DL_SEQ: AtomicU64 = AtomicU64::new(1);

fn next_download_id() -> String {
    let seq = DL_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("dl-{seq}")
}

/// Parameters required to enqueue a new offline media download.
#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub id: Option<String>,
    pub plugin_id: String,
    pub native_id: String,
    pub title: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub kind: DownloadKind,
    pub source_url: String,
    pub headers: BTreeMap<String, String>,
    pub subtitles: Vec<SubtitleTrack>,
}

type ActiveDownloadMap = HashMap<String, (CancellationToken, JoinHandle<()>)>;

/// Concurrent offline download orchestrator backed by SQLite state (`downloads`, `download_segments`).
#[derive(Clone)]
pub struct DownloadManager {
    storage: Arc<SqliteStorage>,
    http: reqwest::Client,
    event_sink: Arc<dyn EventSink>,
    downloads_root: PathBuf,
    queue_semaphore: Arc<Semaphore>,
    active: Arc<Mutex<ActiveDownloadMap>>,
}

impl std::fmt::Debug for DownloadManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadManager")
            .field("downloads_root", &self.downloads_root)
            .finish_non_exhaustive()
    }
}

pub(crate) fn apply_headers(
    mut builder: reqwest::RequestBuilder,
    headers: &BTreeMap<String, String>,
) -> reqwest::RequestBuilder {
    let mut map = HeaderMap::new();
    for (k, v) in headers {
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_str(v),
        ) {
            map.insert(name, val);
        }
    }
    if !map.is_empty() {
        builder = builder.headers(map);
    }
    builder
}

fn subtitle_extension(format: SubtitleFormat) -> &'static str {
    match format {
        SubtitleFormat::Srt => "srt",
        SubtitleFormat::Ass => "ass",
        SubtitleFormat::Ssa => "ssa",
        SubtitleFormat::Vtt => "vtt",
    }
}

pub(crate) async fn bundle_subtitles(
    client: &reqwest::Client,
    subtitles: &[SubtitleTrack],
    fallback_headers: &BTreeMap<String, String>,
    local_dir: &Path,
) -> VesselResult<()> {
    for (idx, track) in subtitles.iter().enumerate() {
        let ext = subtitle_extension(track.format);
        let file_path = local_dir.join(format!("sub_{idx}.{ext}"));
        match &track.source {
            SubtitleSource::InlineContent { content } => {
                fs::write(&file_path, content.as_bytes())
                    .await
                    .map_err(|err| VesselError::Storage(format!("write inline subtitle: {err}")))?;
            }
            SubtitleSource::Url { url, headers } => {
                let effective = if headers.is_empty() {
                    fallback_headers
                } else {
                    headers
                };
                let resp = apply_headers(client.get(url), effective)
                    .send()
                    .await
                    .map_err(|err| {
                        VesselError::Network(format!("subtitle fetch '{url}' failed: {err}"))
                    })?;
                if !resp.status().is_success() {
                    return Err(VesselError::Network(format!(
                        "subtitle fetch '{url}' returned HTTP {}",
                        resp.status()
                    )));
                }
                let bytes = resp.bytes().await.map_err(|err| {
                    VesselError::Network(format!("read subtitle bytes '{url}': {err}"))
                })?;
                fs::write(&file_path, &bytes)
                    .await
                    .map_err(|err| VesselError::Storage(format!("write subtitle file: {err}")))?;
            }
        }
    }
    Ok(())
}

impl DownloadManager {
    /// Creates a new [`DownloadManager`] storing offline bundles inside `downloads_root`.
    pub fn new(
        storage: Arc<SqliteStorage>,
        event_sink: Arc<dyn EventSink>,
        downloads_root: impl Into<PathBuf>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            storage,
            http,
            event_sink,
            downloads_root: downloads_root.into(),
            queue_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS)),
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Persists a download job as `Queued` and schedules it against the 2-slot queue semaphore.
    pub async fn enqueue(&self, req: DownloadRequest) -> VesselResult<StoredDownload> {
        let id = req.id.unwrap_or_else(next_download_id);
        let local_dir = self.downloads_root.join(&id);
        let record = StoredDownload {
            id: id.clone(),
            plugin_id: req.plugin_id,
            native_id: req.native_id,
            title: req.title,
            season: req.season,
            episode: req.episode,
            kind: req.kind,
            source_url: req.source_url,
            headers: req.headers,
            local_dir: local_dir.to_string_lossy().into_owned(),
            local_media_path: None,
            status: DownloadStatus::Queued,
            bytes_downloaded: 0,
            total_bytes: None,
            segments_done: 0,
            segments_total: 0,
            subtitles: req.subtitles,
            error_msg: None,
            created_at: String::new(),
            updated_at: String::new(),
        };

        self.storage.upsert_download(&record).await?;
        self.event_sink
            .emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
                download_id: id,
                status: DownloadStatus::Queued,
                bytes_downloaded: 0,
                total_bytes: None,
                segments_done: 0,
                segments_total: 0,
            }));

        self.spawn_worker(record.clone()).await;
        Ok(record)
    }

    /// Cancels an active worker and transitions its status to `Paused` while preserving partial files.
    pub async fn pause(&self, download_id: &str) -> VesselResult<()> {
        let entry = self.active.lock().await.remove(download_id);
        if let Some((token, handle)) = entry {
            token.cancel();
            let _ = handle.await;
            return Ok(());
        }
        if let Some(existing) = self.storage.get_download(download_id).await?
            && existing.status != DownloadStatus::Completed
        {
            self.storage
                .update_download_progress(
                    download_id,
                    DownloadStatus::Paused,
                    existing.bytes_downloaded,
                    existing.total_bytes,
                    existing.segments_done,
                    existing.segments_total,
                    existing.local_media_path.as_deref(),
                    None,
                )
                .await?;
            self.event_sink
                .emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
                    download_id: download_id.to_string(),
                    status: DownloadStatus::Paused,
                    bytes_downloaded: existing.bytes_downloaded,
                    total_bytes: existing.total_bytes,
                    segments_done: existing.segments_done,
                    segments_total: existing.segments_total,
                }));
        }
        Ok(())
    }

    /// Resumes a paused or failed download from its SQLite segment checkpoint or partial file offset.
    pub async fn resume(&self, download_id: &str) -> VesselResult<StoredDownload> {
        let existing = self
            .storage
            .get_download(download_id)
            .await?
            .ok_or_else(|| {
                VesselError::NotFound(format!("download '{download_id}' does not exist"))
            })?;
        if existing.status == DownloadStatus::Completed {
            return Ok(existing);
        }

        self.storage
            .update_download_progress(
                download_id,
                DownloadStatus::Queued,
                existing.bytes_downloaded,
                existing.total_bytes,
                existing.segments_done,
                existing.segments_total,
                None,
                None,
            )
            .await?;

        let refreshed = self
            .storage
            .get_download(download_id)
            .await?
            .unwrap_or(existing);
        self.spawn_worker(refreshed.clone()).await;
        Ok(refreshed)
    }

    /// Atomically cancels any active download, deletes its local directory, and removes DB rows.
    pub async fn delete(&self, download_id: &str) -> VesselResult<()> {
        if let Some((token, handle)) = self.active.lock().await.remove(download_id) {
            token.cancel();
            let _ = handle.await;
        }
        let existing = self.storage.get_download(download_id).await?;
        let dir_to_remove = existing
            .map(|r| PathBuf::from(r.local_dir))
            .unwrap_or_else(|| self.downloads_root.join(download_id));

        if fs::metadata(&dir_to_remove).await.is_ok() {
            fs::remove_dir_all(&dir_to_remove)
                .await
                .map_err(|err| VesselError::Storage(format!("remove download directory: {err}")))?;
        }
        self.storage.delete_download(download_id).await
    }

    /// Lists all recorded downloads ordered newest-first.
    pub async fn list(&self) -> VesselResult<Vec<StoredDownload>> {
        self.storage.list_downloads().await
    }

    /// Retrieves a single download record by ID.
    pub async fn get(&self, download_id: &str) -> VesselResult<Option<StoredDownload>> {
        self.storage.get_download(download_id).await
    }

    async fn spawn_worker(&self, record: StoredDownload) {
        let token = CancellationToken::new();
        let manager = self.clone();
        let worker_token = token.clone();
        let dl_id = record.id.clone();

        let handle = tokio::spawn(async move {
            let permit = tokio::select! {
                _ = worker_token.cancelled() => {
                    let _ = manager.storage.update_download_progress(
                        &record.id,
                        DownloadStatus::Paused,
                        record.bytes_downloaded,
                        record.total_bytes,
                        record.segments_done,
                        record.segments_total,
                        None,
                        None,
                    ).await;
                    manager.active.lock().await.remove(&record.id);
                    return;
                }
                res = Arc::clone(&manager.queue_semaphore).acquire_owned() => match res {
                    Ok(p) => p,
                    Err(_) => {
                        manager.active.lock().await.remove(&record.id);
                        return;
                    }
                }
            };

            let result = match record.kind {
                DownloadKind::Direct => {
                    direct::run_direct_download(
                        &manager.http,
                        &manager.storage,
                        &manager.event_sink,
                        &record,
                        &worker_token,
                    )
                    .await
                }
                DownloadKind::Hls => {
                    hls::run_hls_download(
                        &manager.http,
                        &manager.storage,
                        &manager.event_sink,
                        &record,
                        &worker_token,
                    )
                    .await
                }
            };
            drop(permit);

            if let Err(err) = result
                && err != VesselError::Cancelled
            {
                let latest = manager
                    .storage
                    .get_download(&record.id)
                    .await
                    .ok()
                    .flatten();
                let (bytes, total, done, total_seg) = latest
                    .map(|d| {
                        (
                            d.bytes_downloaded,
                            d.total_bytes,
                            d.segments_done,
                            d.segments_total,
                        )
                    })
                    .unwrap_or((
                        record.bytes_downloaded,
                        record.total_bytes,
                        record.segments_done,
                        record.segments_total,
                    ));
                let msg = err.to_string();
                let _ = manager
                    .storage
                    .update_download_progress(
                        &record.id,
                        DownloadStatus::Failed,
                        bytes,
                        total,
                        done,
                        total_seg,
                        None,
                        Some(&msg),
                    )
                    .await;
                manager
                    .event_sink
                    .emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
                        download_id: record.id.clone(),
                        status: DownloadStatus::Failed,
                        bytes_downloaded: bytes,
                        total_bytes: total,
                        segments_done: done,
                        segments_total: total_seg,
                    }));
            }

            manager.active.lock().await.remove(&record.id);
        });

        let mut guard = self.active.lock().await;
        if let Some((old_token, old_handle)) = guard.insert(dl_id, (token, handle)) {
            old_token.cancel();
            old_handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use aes::Aes128;
    use aes::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
    use axum::Router;
    use axum::body::Body;
    use axum::extract::{Path as AxumPath, State};
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::get;
    use tokio::net::TcpListener;
    use vessel_domain::{
        DownloadStatus, EngineEvent, EventSink, SubtitleFormat, SubtitleSource, SubtitleTrack,
    };
    use vessel_storage::{DownloadKind, SqliteStorage, StorageActor};

    use super::{DownloadManager, DownloadRequest};

    #[derive(Default)]
    struct CollectorSink {
        events: Mutex<Vec<EngineEvent>>,
    }

    impl EventSink for CollectorSink {
        fn emit(&self, event: EngineEvent) {
            self.events.lock().expect("lock events").push(event);
        }
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("vessel-dl-{name}-{nanos}"))
    }

    fn encrypt_aes128(plain: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Vec<u8> {
        let pad = 16 - (plain.len() % 16);
        let mut buf = vec![0u8; plain.len() + pad];
        buf[..plain.len()].copy_from_slice(plain);
        let ct = cbc::Encryptor::<Aes128>::new(key.into(), iv.into())
            .encrypt_padded::<Pkcs7>(&mut buf, plain.len())
            .expect("encrypt pkcs7");
        let len = ct.len();
        buf.truncate(len);
        buf
    }

    async fn wait_until_status(
        manager: &DownloadManager,
        id: &str,
        wanted: DownloadStatus,
    ) -> vessel_storage::StoredDownload {
        for _ in 0..100 {
            if let Some(dl) = manager.get(id).await.expect("get")
                && dl.status == wanted
            {
                return dl;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("timed out waiting for status {wanted:?} on {id}");
    }

    #[tokio::test]
    async fn direct_download_supports_http_range_resume_and_delete_cleanup() {
        let payload = b"ANTHOLOGY_DIRECT_VIDEO_PAYLOAD_0123456789ABCDEF".to_vec();
        let saw_range = Arc::new(Mutex::new(false));
        let payload_clone = payload.clone();
        let saw_range_clone = Arc::clone(&saw_range);

        let app = Router::new().route(
            "/video.mp4",
            get(move |headers: HeaderMap| {
                let data = payload_clone.clone();
                let saw = Arc::clone(&saw_range_clone);
                async move {
                    if let Some(range_val) = headers
                        .get(axum::http::header::RANGE)
                        .and_then(|v| v.to_str().ok())
                        && let Some(start_str) = range_val
                            .strip_prefix("bytes=")
                            .and_then(|r| r.strip_suffix('-'))
                        && let Ok(start) = start_str.parse::<usize>()
                        && start < data.len()
                    {
                        *saw.lock().expect("lock saw") = true;
                        let total = data.len();
                        let slice = data[start..].to_vec();
                        let content_range = format!("bytes {start}-{}/{total}", total - 1);
                        return (
                            StatusCode::PARTIAL_CONTENT,
                            [(axum::http::header::CONTENT_RANGE, content_range)],
                            Body::from(slice),
                        )
                            .into_response();
                    }
                    (StatusCode::OK, Body::from(data)).into_response()
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let root = temp_dir("direct");
        let actor = StorageActor::open_in_memory().await.expect("db");
        let storage = Arc::new(SqliteStorage::new(actor));
        let sink = Arc::new(CollectorSink::default());
        let manager = DownloadManager::new(storage, sink, root.clone());

        // Pre-seed a partial file simulating an interrupted download at byte 10.
        let part_dir = root.join("dl-range");
        tokio::fs::create_dir_all(&part_dir).await.expect("mkdir");
        tokio::fs::write(part_dir.join("media.part"), &payload[..10])
            .await
            .expect("seed part");

        let rec = manager
            .enqueue(DownloadRequest {
                id: Some("dl-range".into()),
                plugin_id: "anthology".into(),
                native_id: "ep1".into(),
                title: "Anthology Ep 1".into(),
                season: Some(1),
                episode: Some(1),
                kind: DownloadKind::Direct,
                source_url: format!("http://{addr}/video.mp4"),
                headers: BTreeMap::new(),
                subtitles: vec![SubtitleTrack {
                    id: Some("sub-en".into()),
                    source: SubtitleSource::InlineContent {
                        content: "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHello".into(),
                    },
                    lang: "en".into(),
                    label: "English".into(),
                    format: SubtitleFormat::Vtt,
                    is_default: true,
                }],
            })
            .await
            .expect("enqueue");
        assert_eq!(rec.id, "dl-range");

        let done = wait_until_status(&manager, "dl-range", DownloadStatus::Completed).await;
        assert!(
            *saw_range.lock().expect("lock saw"),
            "expected Range 206 resume"
        );
        let media_path = done.local_media_path.expect("local media path");
        let saved = tokio::fs::read(&media_path).await.expect("read saved");
        assert_eq!(saved, payload);
        assert!(part_dir.join("sub_0.vtt").exists());

        manager.delete("dl-range").await.expect("delete");
        assert!(!part_dir.exists());
        assert!(manager.get("dl-range").await.expect("get").is_none());

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn hls_download_decrypts_aes128_segments_in_parallel_and_writes_local_m3u8() {
        let key: [u8; 16] = *b"ANThologyAesKey!";
        let explicit_iv: [u8; 16] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ];
        let seg0_plain = b"SEGMENT_0_TS_BYTES_FOR_ANTHOLOGY".to_vec();
        let seg1_plain = b"SEGMENT_1_TS_BYTES_FOR_ANTHOLOGY".to_vec();
        let seg0_ct = encrypt_aes128(&seg0_plain, &key, &explicit_iv);
        let seg1_ct = encrypt_aes128(&seg1_plain, &key, &explicit_iv);

        #[derive(Clone)]
        struct HlsMockState {
            key: [u8; 16],
            seg0: Vec<u8>,
            seg1: Vec<u8>,
        }

        let state = HlsMockState {
            key,
            seg0: seg0_ct,
            seg1: seg1_ct,
        };

        let app = Router::new()
            .route(
                "/master.m3u8",
                get(|| async {
                    "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=500000\nlow.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2500000\nhigh.m3u8\n"
                }),
            )
            .route(
                "/high.m3u8",
                get(|| async {
                    "#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXT-X-MEDIA-SEQUENCE:10\n#EXT-X-KEY:METHOD=AES-128,URI=\"/enc.key\",IV=0x0102030405060708090a0b0c0d0e0f10\n#EXTINF:5.5,\nseg0.ts\n#EXTINF:6.0,\nseg1.ts\n#EXT-X-ENDLIST\n"
                }),
            )
            .route(
                "/enc.key",
                get(|State(st): State<HlsMockState>| async move { st.key.to_vec() }),
            )
            .route(
                "/{seg}",
                get(
                    |AxumPath(seg): AxumPath<String>, State(st): State<HlsMockState>| async move {
                        match seg.as_str() {
                            "seg0.ts" => (StatusCode::OK, st.seg0).into_response(),
                            "seg1.ts" => (StatusCode::OK, st.seg1).into_response(),
                            _ => StatusCode::NOT_FOUND.into_response(),
                        }
                    },
                ),
            )
            .with_state(state);

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let root = temp_dir("hls");
        let actor = StorageActor::open_in_memory().await.expect("db");
        let storage = Arc::new(SqliteStorage::new(actor));
        let sink = Arc::new(CollectorSink::default());
        let manager = DownloadManager::new(Arc::clone(&storage), sink, root.clone());

        manager
            .enqueue(DownloadRequest {
                id: Some("dl-hls".into()),
                plugin_id: "anthology".into(),
                native_id: "ep2".into(),
                title: "Anthology Ep 2".into(),
                season: Some(1),
                episode: Some(2),
                kind: DownloadKind::Hls,
                source_url: format!("http://{addr}/master.m3u8"),
                headers: BTreeMap::new(),
                subtitles: Vec::new(),
            })
            .await
            .expect("enqueue hls");

        let done = wait_until_status(&manager, "dl-hls", DownloadStatus::Completed).await;
        assert_eq!(done.segments_done, 2);
        assert_eq!(done.segments_total, 2);

        let local_m3u8_path = std::path::PathBuf::from(done.local_media_path.expect("local m3u8"));
        let manifest = tokio::fs::read_to_string(&local_m3u8_path)
            .await
            .expect("read local.m3u8");
        assert!(!manifest.contains("#EXT-X-KEY"));
        assert!(manifest.contains("seg_00000.ts"));
        assert!(manifest.contains("seg_00001.ts"));

        let bundle_dir = local_m3u8_path.parent().expect("dir");
        assert_eq!(
            tokio::fs::read(bundle_dir.join("seg_00000.ts"))
                .await
                .expect("seg0"),
            seg0_plain
        );
        assert_eq!(
            tokio::fs::read(bundle_dir.join("seg_00001.ts"))
                .await
                .expect("seg1"),
            seg1_plain
        );

        let db_segments = storage
            .list_download_segments("dl-hls")
            .await
            .expect("segs");
        assert_eq!(db_segments.len(), 2);
        assert!(db_segments.iter().all(|s| s.completed));

        let _ = tokio::fs::remove_dir_all(&root).await;
    }
}
