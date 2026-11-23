//! SQLite persistence for offline downloads and resumable HLS segment progress.

use std::collections::BTreeMap;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use vessel_domain::{DownloadStatus, SubtitleTrack, VesselError, VesselResult};

use crate::{SqliteStorage, map_sql_err};

/// Transport flavor stored in `downloads.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadKind {
    Direct,
    Hls,
}

impl DownloadKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Hls => "hls",
        }
    }

    fn parse(raw: &str) -> VesselResult<Self> {
        match raw {
            "direct" => Ok(Self::Direct),
            "hls" => Ok(Self::Hls),
            other => Err(VesselError::Storage(format!(
                "unknown download kind '{other}'"
            ))),
        }
    }
}

fn status_as_str(status: DownloadStatus) -> &'static str {
    match status {
        DownloadStatus::Queued => "queued",
        DownloadStatus::Downloading => "downloading",
        DownloadStatus::Paused => "paused",
        DownloadStatus::Completed => "completed",
        DownloadStatus::Failed => "failed",
    }
}

fn parse_status(raw: &str) -> VesselResult<DownloadStatus> {
    match raw {
        "queued" => Ok(DownloadStatus::Queued),
        "downloading" => Ok(DownloadStatus::Downloading),
        "paused" => Ok(DownloadStatus::Paused),
        "completed" => Ok(DownloadStatus::Completed),
        "failed" => Ok(DownloadStatus::Failed),
        other => Err(VesselError::Storage(format!(
            "unknown download status '{other}'"
        ))),
    }
}

/// Persisted offline download job matching `downloads`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredDownload {
    pub id: String,
    pub plugin_id: String,
    pub native_id: String,
    pub title: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub kind: DownloadKind,
    pub source_url: String,
    pub headers: BTreeMap<String, String>,
    pub local_dir: String,
    pub local_media_path: Option<String>,
    pub status: DownloadStatus,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub segments_done: u32,
    pub segments_total: u32,
    pub subtitles: Vec<SubtitleTrack>,
    pub error_msg: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Persisted HLS segment record matching `download_segments`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDownloadSegment {
    pub download_id: String,
    pub seq: u32,
    pub url: String,
    pub file_name: String,
    pub completed: bool,
}

fn decode_download_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(StoredDownloadRaw,)> {
    Ok((StoredDownloadRaw {
        id: row.get(0)?,
        plugin_id: row.get(1)?,
        native_id: row.get(2)?,
        title: row.get(3)?,
        season: row.get(4)?,
        episode: row.get(5)?,
        kind: row.get(6)?,
        source_url: row.get(7)?,
        headers_json: row.get(8)?,
        local_dir: row.get(9)?,
        local_media_path: row.get(10)?,
        status: row.get(11)?,
        bytes_downloaded: row.get(12)?,
        total_bytes: row.get(13)?,
        segments_done: row.get(14)?,
        segments_total: row.get(15)?,
        subtitles_json: row.get(16)?,
        error_msg: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    },))
}

struct StoredDownloadRaw {
    id: String,
    plugin_id: String,
    native_id: String,
    title: String,
    season: Option<u32>,
    episode: Option<u32>,
    kind: String,
    source_url: String,
    headers_json: String,
    local_dir: String,
    local_media_path: Option<String>,
    status: String,
    bytes_downloaded: i64,
    total_bytes: Option<i64>,
    segments_done: u32,
    segments_total: u32,
    subtitles_json: String,
    error_msg: Option<String>,
    created_at: String,
    updated_at: String,
}

impl StoredDownloadRaw {
    fn into_domain(self) -> VesselResult<StoredDownload> {
        let headers: BTreeMap<String, String> = serde_json::from_str(&self.headers_json)
            .map_err(|err| VesselError::Storage(format!("decode download headers: {err}")))?;
        let subtitles: Vec<SubtitleTrack> = serde_json::from_str(&self.subtitles_json)
            .map_err(|err| VesselError::Storage(format!("decode download subtitles: {err}")))?;
        Ok(StoredDownload {
            id: self.id,
            plugin_id: self.plugin_id,
            native_id: self.native_id,
            title: self.title,
            season: self.season,
            episode: self.episode,
            kind: DownloadKind::parse(&self.kind)?,
            source_url: self.source_url,
            headers,
            local_dir: self.local_dir,
            local_media_path: self.local_media_path,
            status: parse_status(&self.status)?,
            bytes_downloaded: u64::try_from(self.bytes_downloaded.max(0)).unwrap_or(0),
            total_bytes: self.total_bytes.and_then(|v| u64::try_from(v.max(0)).ok()),
            segments_done: self.segments_done,
            segments_total: self.segments_total,
            subtitles,
            error_msg: self.error_msg,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

impl SqliteStorage {
    /// Creates or updates a download task record in `downloads`.
    pub async fn upsert_download(&self, item: &StoredDownload) -> VesselResult<()> {
        let dl = item.clone();
        let headers_json = serde_json::to_string(&dl.headers)
            .map_err(|err| VesselError::Storage(format!("encode download headers: {err}")))?;
        let subtitles_json = serde_json::to_string(&dl.subtitles)
            .map_err(|err| VesselError::Storage(format!("encode download subtitles: {err}")))?;
        let bytes_downloaded = i64::try_from(dl.bytes_downloaded).unwrap_or(i64::MAX);
        let total_bytes = dl.total_bytes.map(|v| i64::try_from(v).unwrap_or(i64::MAX));

        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO downloads (
                             id, plugin_id, native_id, title, season, episode, kind, source_url,
                             headers_json, local_dir, local_media_path, status, bytes_downloaded,
                             total_bytes, segments_done, segments_total, subtitles_json, error_msg,
                             created_at, updated_at
                         ) VALUES (
                             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                             ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                             COALESCE(NULLIF(?19, ''), strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                             COALESCE(NULLIF(?20, ''), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                         )
                         ON CONFLICT(id) DO UPDATE SET
                             local_media_path = excluded.local_media_path,
                             status = excluded.status,
                             bytes_downloaded = excluded.bytes_downloaded,
                             total_bytes = excluded.total_bytes,
                             segments_done = excluded.segments_done,
                             segments_total = excluded.segments_total,
                             subtitles_json = excluded.subtitles_json,
                             error_msg = excluded.error_msg,
                             updated_at = excluded.updated_at",
                        rusqlite::params![
                            dl.id,
                            dl.plugin_id,
                            dl.native_id,
                            dl.title,
                            dl.season,
                            dl.episode,
                            dl.kind.as_str(),
                            dl.source_url,
                            headers_json,
                            dl.local_dir,
                            dl.local_media_path,
                            status_as_str(dl.status),
                            bytes_downloaded,
                            total_bytes,
                            dl.segments_done,
                            dl.segments_total,
                            subtitles_json,
                            dl.error_msg,
                            dl.created_at,
                            dl.updated_at,
                        ],
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Reads a single download task by ID.
    pub async fn get_download(&self, download_id: &str) -> VesselResult<Option<StoredDownload>> {
        let id = download_id.to_owned();
        self.actor()
            .execute(move |db| {
                let row = db
                    .conn()
                    .query_row(
                        "SELECT id, plugin_id, native_id, title, season, episode, kind, source_url,
                                headers_json, local_dir, local_media_path, status, bytes_downloaded,
                                total_bytes, segments_done, segments_total, subtitles_json, error_msg,
                                created_at, updated_at
                         FROM downloads WHERE id = ?1",
                        [&id],
                        decode_download_row,
                    )
                    .optional()
                    .map_err(map_sql_err)?;
                row.map(|(raw,)| raw.into_domain()).transpose()
            })
            .await
    }

    /// Lists all recorded download tasks ordered by newest update first.
    pub async fn list_downloads(&self) -> VesselResult<Vec<StoredDownload>> {
        self.actor()
            .execute(|db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT id, plugin_id, native_id, title, season, episode, kind, source_url,
                                headers_json, local_dir, local_media_path, status, bytes_downloaded,
                                total_bytes, segments_done, segments_total, subtitles_json, error_msg,
                                created_at, updated_at
                         FROM downloads ORDER BY updated_at DESC",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([], decode_download_row)
                    .map_err(map_sql_err)?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r.map_err(map_sql_err)?.0.into_domain()?);
                }
                Ok(out)
            })
            .await
    }

    /// Updates status, progress counters, optional media path, and error message for `download_id`.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_download_progress(
        &self,
        download_id: &str,
        status: DownloadStatus,
        bytes_downloaded: u64,
        total_bytes: Option<u64>,
        segments_done: u32,
        segments_total: u32,
        local_media_path: Option<&str>,
        error_msg: Option<&str>,
    ) -> VesselResult<()> {
        let id = download_id.to_owned();
        let bytes = i64::try_from(bytes_downloaded).unwrap_or(i64::MAX);
        let total = total_bytes.map(|v| i64::try_from(v).unwrap_or(i64::MAX));
        let path = local_media_path.map(str::to_owned);
        let err_s = error_msg.map(str::to_owned);

        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "UPDATE downloads
                         SET status = ?1,
                             bytes_downloaded = ?2,
                             total_bytes = COALESCE(?3, total_bytes),
                             segments_done = ?4,
                             segments_total = ?5,
                             local_media_path = COALESCE(?6, local_media_path),
                             error_msg = ?7,
                             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                         WHERE id = ?8",
                        (
                            status_as_str(status),
                            bytes,
                            total,
                            segments_done,
                            segments_total,
                            &path,
                            &err_s,
                            &id,
                        ),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Deletes a download record (cascading `download_segments` via foreign keys).
    pub async fn delete_download(&self, download_id: &str) -> VesselResult<()> {
        let id = download_id.to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute("DELETE FROM downloads WHERE id = ?1", [&id])
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Persists the initial list of HLS segments for resumable tracking.
    pub async fn seed_download_segments(
        &self,
        download_id: &str,
        segments: Vec<StoredDownloadSegment>,
    ) -> VesselResult<()> {
        let id = download_id.to_owned();
        self.actor()
            .execute(move |db| {
                let conn = db.conn();
                conn.execute_batch("BEGIN IMMEDIATE").map_err(map_sql_err)?;
                let res = (|| -> VesselResult<()> {
                    let mut stmt = conn
                        .prepare(
                            "INSERT OR IGNORE INTO download_segments (download_id, seq, url, file_name, completed)
                             VALUES (?1, ?2, ?3, ?4, ?5)",
                        )
                        .map_err(map_sql_err)?;
                    for seg in segments {
                        stmt.execute((
                            &id,
                            seg.seq,
                            &seg.url,
                            &seg.file_name,
                            i32::from(seg.completed),
                        ))
                        .map_err(map_sql_err)?;
                    }
                    Ok(())
                })();
                match res {
                    Ok(()) => {
                        conn.execute_batch("COMMIT").map_err(map_sql_err)?;
                        Ok(())
                    }
                    Err(err) => {
                        let _ = conn.execute_batch("ROLLBACK");
                        Err(err)
                    }
                }
            })
            .await
    }

    /// Lists all segments for `download_id` ordered by `seq ASC`.
    pub async fn list_download_segments(
        &self,
        download_id: &str,
    ) -> VesselResult<Vec<StoredDownloadSegment>> {
        let id = download_id.to_owned();
        self.actor()
            .execute(move |db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT download_id, seq, url, file_name, completed
                         FROM download_segments WHERE download_id = ?1 ORDER BY seq ASC",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([&id], |row| {
                        let completed: i64 = row.get(4)?;
                        Ok(StoredDownloadSegment {
                            download_id: row.get(0)?,
                            seq: row.get(1)?,
                            url: row.get(2)?,
                            file_name: row.get(3)?,
                            completed: completed != 0,
                        })
                    })
                    .map_err(map_sql_err)?;
                let mut out = Vec::new();
                for item in rows {
                    out.push(item.map_err(map_sql_err)?);
                }
                Ok(out)
            })
            .await
    }

    /// Marks one HLS segment complete after decryption & flush to disk.
    pub async fn mark_segment_completed(&self, download_id: &str, seq: u32) -> VesselResult<()> {
        let id = download_id.to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "UPDATE download_segments SET completed = 1 WHERE download_id = ?1 AND seq = ?2",
                        (&id, seq),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }
}
