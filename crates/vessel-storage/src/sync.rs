//! SQLite tombstone snapshot export and Last-Write-Wins (LWW) merge for cross-device sync.

use std::collections::BTreeMap;

use rusqlite::{OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use vessel_domain::{
    LibraryItem, MediaRef, PlaybackProgress, PluginId, VesselError, VesselResult,
};

use crate::library::{
    RawLibraryRow, i64_to_u64, library_status_as_str, media_type_as_str, u64_to_i64,
};
use crate::{SqliteStorage, map_sql_err};

/// Current schema version for Google Drive incremental tombstone snapshots.
pub const CLOUD_SYNC_SCHEMA_VERSION: u32 = 2;

/// Settings keys that remain strictly local to one physical device.
const DEVICE_LOCAL_SETTING_KEYS: &[&str] = &[
    "custom_download_dir",
    "vessel.keyring_key_index",
    "sync.google_drive.last_synced_at",
    "sync.google_drive.last_error",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncLibraryRecord {
    pub item: LibraryItem,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_device_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgressRecord {
    pub progress: PlaybackProgress,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_device_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOverrideRecord {
    pub plugin_id: String,
    pub native_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_id: Option<String>,
    pub is_disabled: bool,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_device_id: Option<String>,
}

/// Complete portable sync state exchanged with Google Drive `appDataFolder`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncSnapshot {
    pub schema_version: u32,
    pub device_id: String,
    pub exported_at: String,
    #[serde(default)]
    pub settings: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub library: Vec<SyncLibraryRecord>,
    #[serde(default)]
    pub watch_progress: Vec<SyncProgressRecord>,
    #[serde(default)]
    pub metadata_overrides: Vec<SyncOverrideRecord>,
}

/// Summary of applied changes during a cloud snapshot merge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncMergeStats {
    pub library_applied: usize,
    pub progress_applied: usize,
    pub overrides_applied: usize,
}

fn effective_timestamp<'a>(updated_at: &'a str, deleted_at: Option<&'a str>) -> &'a str {
    match deleted_at {
        Some(del) if del > updated_at => del,
        _ => updated_at,
    }
}

fn generate_device_id() -> String {
    let mut bytes = [0u8; 16];
    let _ = getrandom::fill(&mut bytes);
    let mut hex = String::with_capacity(32);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{b:02x}");
    }
    format!("dev-{hex}")
}

struct RawSyncProgressRow {
    plugin_id: String,
    native_id: String,
    stream_id: String,
    canonical_id: Option<String>,
    season: u32,
    episode: u32,
    absolute_episode: Option<u32>,
    position_ms: i64,
    duration_ms: i64,
    completed: i64,
    last_watched_at: String,
    title: Option<String>,
    poster_url: Option<String>,
    backdrop_url: Option<String>,
    episode_title: Option<String>,
    updated_at: String,
    deleted_at: Option<String>,
    origin_device_id: Option<String>,
}

impl RawSyncProgressRow {
    fn from_sql(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            plugin_id: row.get(0)?,
            native_id: row.get(1)?,
            stream_id: row.get(2)?,
            canonical_id: row.get(3)?,
            season: row.get(4)?,
            episode: row.get(5)?,
            absolute_episode: row.get(6)?,
            position_ms: row.get(7)?,
            duration_ms: row.get(8)?,
            completed: row.get(9)?,
            last_watched_at: row.get(10)?,
            title: row.get(11)?,
            poster_url: row.get(12)?,
            backdrop_url: row.get(13)?,
            episode_title: row.get(14)?,
            updated_at: row.get(15)?,
            deleted_at: row.get(16)?,
            origin_device_id: row.get(17)?,
        })
    }

    fn into_record(self) -> VesselResult<SyncProgressRecord> {
        let media = MediaRef::new(PluginId::new(self.plugin_id)?, self.native_id)?;
        let progress = PlaybackProgress {
            media,
            stream_id: self.stream_id,
            canonical_id: self.canonical_id,
            season: self.season,
            episode: self.episode,
            absolute_episode: self.absolute_episode,
            title: self.title,
            poster_url: self.poster_url,
            backdrop_url: self.backdrop_url,
            episode_title: self.episode_title,
            position_ms: i64_to_u64(self.position_ms, "position_ms")?,
            duration_ms: i64_to_u64(self.duration_ms, "duration_ms")?,
            completed: self.completed != 0,
            last_watched_at: self.last_watched_at,
        };
        Ok(SyncProgressRecord {
            progress,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
            origin_device_id: self.origin_device_id,
        })
    }
}

impl SqliteStorage {
    /// Returns the persistent unique identifier for this device, generating one on first run.
    pub async fn get_or_create_sync_device_id(&self) -> VesselResult<String> {
        self.actor()
            .execute(|db| {
                if let Some(existing) = db
                    .conn()
                    .query_row(
                        "SELECT value FROM sync_state WHERE key = 'device_id'",
                        [],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(map_sql_err)?
                {
                    return Ok(existing);
                }

                let device_id = generate_device_id();
                db.conn()
                    .execute(
                        "INSERT INTO sync_state (key, value, updated_at)
                         VALUES ('device_id', ?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                        [&device_id],
                    )
                    .map_err(map_sql_err)?;
                Ok(device_id)
            })
            .await
    }

    /// Reads a sync state key (`last_synced_at`, `auto_sync`, etc.) from `sync_state`.
    pub async fn get_sync_state(&self, key: &str) -> VesselResult<Option<String>> {
        let key = key.trim().to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .query_row("SELECT value FROM sync_state WHERE key = ?1", [&key], |r| {
                        r.get(0)
                    })
                    .optional()
                    .map_err(map_sql_err)
            })
            .await
    }

    /// Writes or updates a sync state key in `sync_state`.
    pub async fn set_sync_state(&self, key: &str, value: &str) -> VesselResult<()> {
        let key = key.trim().to_owned();
        let value = value.to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO sync_state (key, value, updated_at)
                         VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                         ON CONFLICT(key) DO UPDATE SET
                             value = excluded.value,
                             updated_at = excluded.updated_at",
                        (&key, &value),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Counts soft-deleted tombstone records across library, progress, and overrides.
    pub async fn count_active_tombstones(&self) -> VesselResult<usize> {
        self.actor()
            .execute(|db| {
                let c1: i64 = db
                    .conn()
                    .query_row(
                        "SELECT COUNT(*) FROM library_items WHERE deleted_at IS NOT NULL",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(map_sql_err)?;
                let c2: i64 = db
                    .conn()
                    .query_row(
                        "SELECT COUNT(*) FROM watch_progress WHERE deleted_at IS NOT NULL",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(map_sql_err)?;
                let c3: i64 = db
                    .conn()
                    .query_row(
                        "SELECT COUNT(*) FROM metadata_overrides WHERE deleted_at IS NOT NULL",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(map_sql_err)?;
                Ok((c1 + c2 + c3).max(0) as usize)
            })
            .await
    }

    /// Exports all active + tombstoned syncable records for Google Drive incremental merge.
    pub async fn export_sync_snapshot(&self) -> VesselResult<CloudSyncSnapshot> {
        let device_id = self.get_or_create_sync_device_id().await?;
        self.actor()
            .execute(move |db| {
                let conn = db.conn();
                let exported_at: String = conn
                    .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ', 'now')", [], |r| {
                        r.get(0)
                    })
                    .map_err(map_sql_err)?;

                let mut settings = BTreeMap::new();
                {
                    let mut stmt = conn
                        .prepare("SELECT key, value_json FROM settings ORDER BY key ASC")
                        .map_err(map_sql_err)?;
                    let rows = stmt
                        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                        .map_err(map_sql_err)?;
                    for row in rows {
                        let (k, raw) = row.map_err(map_sql_err)?;
                        if DEVICE_LOCAL_SETTING_KEYS.contains(&k.as_str()) {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) {
                            settings.insert(k, parsed);
                        }
                    }
                }

                let mut library = Vec::new();
                {
                    let mut stmt = conn
                        .prepare(
                            "SELECT plugin_id, native_id, canonical_id, media_type, is_anime,
                                    title, poster_url, year, status, added_at, updated_at,
                                    deleted_at, origin_device_id
                             FROM library_items
                             ORDER BY plugin_id ASC, native_id ASC",
                        )
                        .map_err(map_sql_err)?;
                    let rows = stmt
                        .query_map([], |row| {
                            Ok((
                                RawLibraryRow::from_sql(row)?,
                                row.get::<_, Option<String>>(11)?,
                                row.get::<_, Option<String>>(12)?,
                            ))
                        })
                        .map_err(map_sql_err)?;
                    for row in rows {
                        let (raw, deleted_at, origin_device_id) = row.map_err(map_sql_err)?;
                        library.push(SyncLibraryRecord {
                            item: raw.into_domain()?,
                            deleted_at,
                            origin_device_id,
                        });
                    }
                }

                let mut watch_progress = Vec::new();
                {
                    let mut stmt = conn
                        .prepare(
                            "SELECT plugin_id, native_id, stream_id, canonical_id, season, episode,
                                    absolute_episode, position_ms, duration_ms, completed, last_watched_at,
                                    title, poster_url, backdrop_url, episode_title,
                                    updated_at, deleted_at, origin_device_id
                             FROM watch_progress
                             ORDER BY plugin_id ASC, native_id ASC, season ASC, episode ASC",
                        )
                        .map_err(map_sql_err)?;
                    let rows = stmt
                        .query_map([], RawSyncProgressRow::from_sql)
                        .map_err(map_sql_err)?;
                    for row in rows {
                        watch_progress.push(row.map_err(map_sql_err)?.into_record()?);
                    }
                }

                let mut metadata_overrides = Vec::new();
                {
                    let mut stmt = conn
                        .prepare(
                            "SELECT plugin_id, native_id, canonical_id, is_disabled,
                                    updated_at, deleted_at, origin_device_id
                             FROM metadata_overrides
                             ORDER BY plugin_id ASC, native_id ASC",
                        )
                        .map_err(map_sql_err)?;
                    let rows = stmt
                        .query_map([], |row| {
                            Ok(SyncOverrideRecord {
                                plugin_id: row.get(0)?,
                                native_id: row.get(1)?,
                                canonical_id: row.get(2)?,
                                is_disabled: row.get::<_, i64>(3)? != 0,
                                updated_at: row.get(4)?,
                                deleted_at: row.get(5)?,
                                origin_device_id: row.get(6)?,
                            })
                        })
                        .map_err(map_sql_err)?;
                    for row in rows {
                        metadata_overrides.push(row.map_err(map_sql_err)?);
                    }
                }

                Ok(CloudSyncSnapshot {
                    schema_version: CLOUD_SYNC_SCHEMA_VERSION,
                    device_id,
                    exported_at,
                    settings,
                    library,
                    watch_progress,
                    metadata_overrides,
                })
            })
            .await
    }

    /// Merges a remote `CloudSyncSnapshot` into local SQLite using LWW + tombstone semantics.
    pub async fn merge_sync_snapshot(
        &self,
        remote: CloudSyncSnapshot,
    ) -> VesselResult<SyncMergeStats> {
        if remote.schema_version != CLOUD_SYNC_SCHEMA_VERSION {
            return Err(VesselError::InvalidInput(format!(
                "unsupported cloud sync schemaVersion: {}",
                remote.schema_version
            )));
        }

        self.actor()
            .execute(move |db| {
                let tx = db.conn_mut().transaction().map_err(map_sql_err)?;
                let mut stats = SyncMergeStats::default();

                for (k, v) in &remote.settings {
                    if DEVICE_LOCAL_SETTING_KEYS.contains(&k.as_str()) {
                        continue;
                    }
                    let json = serde_json::to_string(v)
                        .map_err(|err| VesselError::Storage(format!("encode setting '{k}': {err}")))?;
                    tx.execute(
                        "INSERT INTO settings (key, value_json)
                         VALUES (?1, ?2)
                         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
                        (k, &json),
                    )
                    .map_err(map_sql_err)?;
                }

                for rec in &remote.library {
                    let existing: Option<(String, Option<String>)> = tx
                        .query_row(
                            "SELECT updated_at, deleted_at FROM library_items
                             WHERE plugin_id = ?1 AND native_id = ?2",
                            (
                                rec.item.media.plugin_id.as_str(),
                                &rec.item.media.native_id,
                            ),
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .optional()
                        .map_err(map_sql_err)?;

                    let remote_eff =
                        effective_timestamp(&rec.item.updated_at, rec.deleted_at.as_deref());
                    if let Some((local_upd, local_del)) = existing {
                        let local_eff = effective_timestamp(&local_upd, local_del.as_deref());
                        if local_eff >= remote_eff {
                            continue;
                        }
                    }

                    let row_id = format!("{}:{}", rec.item.media.plugin_id, rec.item.media.native_id);
                    tx.execute(
                        "INSERT INTO library_items (
                             id, plugin_id, native_id, canonical_id, media_type, is_anime,
                             title, poster_url, year, status, added_at, updated_at,
                             deleted_at, origin_device_id
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                         ON CONFLICT(plugin_id, native_id) DO UPDATE SET
                             canonical_id = excluded.canonical_id,
                             media_type = excluded.media_type,
                             is_anime = excluded.is_anime,
                             title = excluded.title,
                             poster_url = excluded.poster_url,
                             year = excluded.year,
                             status = excluded.status,
                             updated_at = excluded.updated_at,
                             deleted_at = excluded.deleted_at,
                             origin_device_id = excluded.origin_device_id",
                        (
                            &row_id,
                            rec.item.media.plugin_id.as_str(),
                            &rec.item.media.native_id,
                            &rec.item.canonical_id,
                            media_type_as_str(rec.item.media_type),
                            i32::from(rec.item.is_anime),
                            &rec.item.title,
                            &rec.item.poster_url,
                            rec.item.year,
                            library_status_as_str(rec.item.status),
                            &rec.item.added_at,
                            &rec.item.updated_at,
                            &rec.deleted_at,
                            &rec.origin_device_id,
                        ),
                    )
                    .map_err(map_sql_err)?;
                    stats.library_applied += 1;
                }

                for rec in &remote.watch_progress {
                    let prog = &rec.progress;
                    let position_ms = u64_to_i64(prog.position_ms, "position_ms")?;
                    let duration_ms = u64_to_i64(prog.duration_ms, "duration_ms")?;

                    let existing: Option<(String, Option<String>)> = tx
                        .query_row(
                            "SELECT updated_at, deleted_at FROM watch_progress
                             WHERE plugin_id = ?1 AND native_id = ?2 AND season = ?3 AND episode = ?4",
                            (
                                prog.media.plugin_id.as_str(),
                                &prog.media.native_id,
                                prog.season,
                                prog.episode,
                            ),
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .optional()
                        .map_err(map_sql_err)?;

                    let remote_eff =
                        effective_timestamp(&rec.updated_at, rec.deleted_at.as_deref());
                    if let Some((local_upd, local_del)) = existing {
                        let local_eff = effective_timestamp(&local_upd, local_del.as_deref());
                        if local_eff >= remote_eff {
                            continue;
                        }
                    }

                    let row_id = format!(
                        "{}:{}:{}:{}",
                        prog.media.plugin_id, prog.media.native_id, prog.season, prog.episode
                    );
                    tx.execute(
                        "INSERT INTO watch_progress (
                             id, plugin_id, native_id, stream_id, canonical_id, season, episode,
                             absolute_episode, position_ms, duration_ms, completed, last_watched_at,
                             title, poster_url, backdrop_url, episode_title,
                             updated_at, deleted_at, origin_device_id
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
                         ON CONFLICT(plugin_id, native_id, season, episode) DO UPDATE SET
                             stream_id = excluded.stream_id,
                             canonical_id = excluded.canonical_id,
                             absolute_episode = excluded.absolute_episode,
                             position_ms = excluded.position_ms,
                             duration_ms = excluded.duration_ms,
                             completed = excluded.completed,
                             last_watched_at = excluded.last_watched_at,
                             title = COALESCE(NULLIF(excluded.title, ''), watch_progress.title),
                             poster_url = COALESCE(NULLIF(excluded.poster_url, ''), watch_progress.poster_url),
                             backdrop_url = COALESCE(NULLIF(excluded.backdrop_url, ''), watch_progress.backdrop_url),
                             episode_title = COALESCE(NULLIF(excluded.episode_title, ''), watch_progress.episode_title),
                             updated_at = excluded.updated_at,
                             deleted_at = excluded.deleted_at,
                             origin_device_id = excluded.origin_device_id",
                        rusqlite::params![
                            &row_id,
                            prog.media.plugin_id.as_str(),
                            &prog.media.native_id,
                            &prog.stream_id,
                            &prog.canonical_id,
                            prog.season,
                            prog.episode,
                            prog.absolute_episode,
                            position_ms,
                            duration_ms,
                            i32::from(prog.completed),
                            &prog.last_watched_at,
                            &prog.title,
                            &prog.poster_url,
                            &prog.backdrop_url,
                            &prog.episode_title,
                            &rec.updated_at,
                            &rec.deleted_at,
                            &rec.origin_device_id,
                        ],
                    )
                    .map_err(map_sql_err)?;
                    stats.progress_applied += 1;
                }

                for ov in &remote.metadata_overrides {
                    let existing: Option<(String, Option<String>)> = tx
                        .query_row(
                            "SELECT updated_at, deleted_at FROM metadata_overrides
                             WHERE plugin_id = ?1 AND native_id = ?2",
                            (&ov.plugin_id, &ov.native_id),
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .optional()
                        .map_err(map_sql_err)?;

                    let remote_eff =
                        effective_timestamp(&ov.updated_at, ov.deleted_at.as_deref());
                    if let Some((local_upd, local_del)) = existing {
                        let local_eff = effective_timestamp(&local_upd, local_del.as_deref());
                        if local_eff >= remote_eff {
                            continue;
                        }
                    }

                    tx.execute(
                        "INSERT INTO metadata_overrides (
                             plugin_id, native_id, canonical_id, is_disabled, updated_at,
                             deleted_at, origin_device_id
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                         ON CONFLICT(plugin_id, native_id) DO UPDATE SET
                             canonical_id = excluded.canonical_id,
                             is_disabled = excluded.is_disabled,
                             updated_at = excluded.updated_at,
                             deleted_at = excluded.deleted_at,
                             origin_device_id = excluded.origin_device_id",
                        (
                            &ov.plugin_id,
                            &ov.native_id,
                            &ov.canonical_id,
                            i32::from(ov.is_disabled),
                            &ov.updated_at,
                            &ov.deleted_at,
                            &ov.origin_device_id,
                        ),
                    )
                    .map_err(map_sql_err)?;
                    stats.overrides_applied += 1;
                }

                tx.commit().map_err(map_sql_err)?;
                Ok(stats)
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use vessel_domain::{
        LibraryItem, LibraryStatus, MediaRef, MediaType, PlaybackProgress, PluginId, StoragePort,
    };

    use super::{CLOUD_SYNC_SCHEMA_VERSION, CloudSyncSnapshot, SyncLibraryRecord};
    use crate::{SqliteStorage, StorageActor};

    fn sample_library(plugin: &PluginId, native: &str, updated_at: &str) -> LibraryItem {
        LibraryItem {
            media: MediaRef::new(plugin.clone(), native).expect("media ref"),
            canonical_id: Some("tmdb:movie:603".into()),
            media_type: MediaType::Movie,
            is_anime: false,
            title: "The Matrix".into(),
            poster_url: None,
            year: Some(1999),
            status: LibraryStatus::Completed,
            added_at: "2026-11-28T09:00:00Z".into(),
            updated_at: updated_at.into(),
        }
    }

    #[tokio::test]
    async fn tombstones_prevent_resurrection_of_older_items_and_allow_newer_readds() {
        let actor = StorageActor::open_in_memory().await.expect("actor");
        let storage = SqliteStorage::new(actor);
        let plugin = PluginId::new("anthology").expect("plugin");

        // Device B adds item at 10:00:00Z
        storage
            .upsert_library(&sample_library(&plugin, "/matrix", "2026-11-28T10:00:00Z"))
            .await
            .expect("upsert");

        // Remote Device A deleted it at 10:05:00Z -> deletion must win!
        let remote_delete = CloudSyncSnapshot {
            schema_version: CLOUD_SYNC_SCHEMA_VERSION,
            device_id: "dev-remote-a".into(),
            exported_at: "2026-11-28T10:06:00Z".into(),
            settings: Default::default(),
            library: vec![SyncLibraryRecord {
                item: sample_library(&plugin, "/matrix", "2026-11-28T10:00:00Z"),
                deleted_at: Some("2026-11-28T10:05:00Z".into()),
                origin_device_id: Some("dev-remote-a".into()),
            }],
            watch_progress: Vec::new(),
            metadata_overrides: Vec::new(),
        };
        let stats = storage
            .merge_sync_snapshot(remote_delete)
            .await
            .expect("merge delete");
        assert_eq!(stats.library_applied, 1);
        assert_eq!(storage.count_active_tombstones().await.expect("tombstones"), 1);
        assert!(
            storage
                .list_library(None)
                .await
                .expect("list library")
                .is_empty()
        );

        // Stale remote snapshot from 10:02:00Z must NOT resurrect the deleted item.
        let stale_add = CloudSyncSnapshot {
            schema_version: CLOUD_SYNC_SCHEMA_VERSION,
            device_id: "dev-remote-c".into(),
            exported_at: "2026-11-28T10:02:00Z".into(),
            settings: Default::default(),
            library: vec![SyncLibraryRecord {
                item: sample_library(&plugin, "/matrix", "2026-11-28T10:02:00Z"),
                deleted_at: None,
                origin_device_id: Some("dev-remote-c".into()),
            }],
            watch_progress: Vec::new(),
            metadata_overrides: Vec::new(),
        };
        let stale_stats = storage
            .merge_sync_snapshot(stale_add)
            .await
            .expect("merge stale");
        assert_eq!(stale_stats.library_applied, 0);
        assert!(storage.list_library(None).await.expect("still empty").is_empty());

        // Newer re-add at 10:10:00Z DOES resurrect it and clears deleted_at.
        let newer_add = CloudSyncSnapshot {
            schema_version: CLOUD_SYNC_SCHEMA_VERSION,
            device_id: "dev-remote-c".into(),
            exported_at: "2026-11-28T10:10:00Z".into(),
            settings: Default::default(),
            library: vec![SyncLibraryRecord {
                item: sample_library(&plugin, "/matrix", "2026-11-28T10:10:00Z"),
                deleted_at: None,
                origin_device_id: Some("dev-remote-c".into()),
            }],
            watch_progress: Vec::new(),
            metadata_overrides: Vec::new(),
        };
        let readd_stats = storage
            .merge_sync_snapshot(newer_add)
            .await
            .expect("merge newer");
        assert_eq!(readd_stats.library_applied, 1);
        assert_eq!(storage.list_library(None).await.expect("readded").len(), 1);
        assert_eq!(storage.count_active_tombstones().await.expect("cleared"), 0);

        // Also ensure progress upsert clears deleted_at cleanly.
        let media = MediaRef::new(plugin, "/matrix").expect("media");
        storage
            .upsert_progress(&PlaybackProgress {
                media,
                stream_id: "/stream/1".into(),
                canonical_id: None,
                season: 1,
                episode: 1,
                absolute_episode: None,
                title: Some("The Matrix".into()),
                poster_url: None,
                backdrop_url: None,
                episode_title: None,
                position_ms: 1000,
                duration_ms: 5000,
                completed: false,
                last_watched_at: "2026-11-28T10:11:00Z".into(),
            })
            .await
            .expect("upsert progress");
        let snap = storage.export_sync_snapshot().await.expect("export");
        assert_eq!(snap.watch_progress.len(), 1);
    }
}
