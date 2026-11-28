//! Offline media downloads, Debrid magnet pre-resolution, and local HTTP playback serving.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use vessel_domain::{
    DebridResolution, DownloadStatus, ResolveDebridRequest, StreamKind, SubtitleSource,
    SubtitleTrack, VesselError, VesselResult,
};
use vessel_downloader::DownloadRequest;
use vessel_storage::{DownloadKind, StoredDownload};

use crate::{LocalPlaybackBundle, VesselEngine, subtitle_ext};

impl VesselEngine {
    /// Enqueues an offline media download (resolving torrent magnets via Debrid when needed).
    pub async fn enqueue_download(&self, mut req: DownloadRequest) -> VesselResult<StoredDownload> {
        if req
            .source_url
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("magnet:?")
        {
            let res = self
                .resolve_torrent_candidate(ResolveDebridRequest {
                    url: req.source_url.clone(),
                    torrent: None,
                    preferred_target: None,
                    allow_uncached_queue: false,
                    season: req.season,
                    episode: req.episode,
                })
                .await?;
            match res {
                DebridResolution::Ready { url, headers, .. } => {
                    req.source_url = url;
                    if !headers.is_empty() {
                        req.headers = headers;
                    }
                    req.kind = DownloadKind::Direct;
                }
                DebridResolution::NeedsConfirmation { provider_label, .. } => {
                    return Err(VesselError::InvalidInput(format!(
                        "Torrent is not cached on {provider_label}. Queue it to Debrid cloud before downloading."
                    )));
                }
                DebridResolution::NoProviderConfigured { message } => {
                    return Err(VesselError::InvalidInput(message));
                }
            }
        }
        self.downloader.enqueue(req).await
    }

    /// Pauses an active or queued download while preserving partial files / segment progress.
    pub async fn pause_download(&self, download_id: &str) -> VesselResult<()> {
        self.downloader.pause(download_id).await
    }

    /// Resumes a paused or failed download from its SQLite checkpoint or HTTP `Range` offset.
    pub async fn resume_download(&self, download_id: &str) -> VesselResult<StoredDownload> {
        self.downloader.resume(download_id).await
    }

    /// Cancels, removes local media/subtitles folder, and deletes DB state for `download_id`.
    pub async fn delete_download(&self, download_id: &str) -> VesselResult<()> {
        self.downloader.delete(download_id).await
    }

    /// Lists all tracked offline downloads.
    pub async fn list_downloads(&self) -> VesselResult<Vec<StoredDownload>> {
        self.downloader.list().await
    }

    /// Reads one download record by ID.
    pub async fn get_download(&self, download_id: &str) -> VesselResult<Option<StoredDownload>> {
        self.downloader.get(download_id).await
    }

    /// Persists drag-and-drop download queue priority ordering and dispatches queued jobs.
    pub async fn reorder_downloads(&self, ordered_ids: &[String]) -> VesselResult<()> {
        self.downloader.reorder(ordered_ids).await
    }

    /// Registers a completed offline download folder on the loopback HTTP proxy for playback.
    pub async fn prepare_local_download_playback(
        &self,
        download_id: &str,
    ) -> VesselResult<LocalPlaybackBundle> {
        let record = self.downloader.get(download_id).await?.ok_or_else(|| {
            VesselError::NotFound(format!("download '{download_id}' does not exist"))
        })?;
        if record.status != DownloadStatus::Completed {
            return Err(VesselError::InvalidInput(format!(
                "download '{download_id}' is not completed yet"
            )));
        }
        let media_path = record.local_media_path.as_deref().ok_or_else(|| {
            VesselError::NotFound(format!("download '{download_id}' has no local media path"))
        })?;
        let root_dir = PathBuf::from(&record.local_dir);
        let media_buf = PathBuf::from(media_path);
        if !tokio::fs::try_exists(&media_buf).await.unwrap_or(false) {
            return Err(VesselError::NotFound(format!(
                "local downloaded file is missing at '{}'",
                media_buf.display()
            )));
        }

        let rel_media = media_buf
            .strip_prefix(&root_dir)
            .ok()
            .or_else(|| media_buf.file_name().map(Path::new))
            .ok_or_else(|| VesselError::Storage("invalid local media path".into()))?;
        let rel_str = rel_media.to_string_lossy().replace('\\', "/");

        let base_url = self.proxy.register_local_dir(&root_dir)?;
        let url = format!("{base_url}/{rel_str}");

        let kind = match record.kind {
            DownloadKind::Hls => StreamKind::Hls,
            DownloadKind::Direct => match media_buf
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .as_deref()
            {
                Some("mkv") => StreamKind::Mkv,
                Some("webm") => StreamKind::Webm,
                Some("ts") => StreamKind::MpegTs,
                Some("m3u8") => StreamKind::Hls,
                _ => StreamKind::Mp4,
            },
        };

        let mut subtitles = Vec::new();
        for (idx, track) in record.subtitles.into_iter().enumerate() {
            let ext = subtitle_ext(track.format);
            let filename = format!("sub_{idx}.{ext}");
            if tokio::fs::try_exists(root_dir.join(&filename))
                .await
                .unwrap_or(false)
            {
                subtitles.push(SubtitleTrack {
                    id: track.id,
                    source: SubtitleSource::Url {
                        url: format!("{base_url}/{filename}"),
                        headers: BTreeMap::new(),
                    },
                    lang: track.lang,
                    label: track.label,
                    format: track.format,
                    is_default: track.is_default,
                });
            } else if matches!(track.source, SubtitleSource::InlineContent { .. }) {
                subtitles.push(track);
            }
        }

        Ok(LocalPlaybackBundle {
            download_id: record.id,
            url,
            kind,
            subtitles,
        })
    }
}
