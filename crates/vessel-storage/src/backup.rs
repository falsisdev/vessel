//! Portable `.backup.vessel` JSON snapshot export and import.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use vessel_domain::{
    LibraryItem, MediaRef, PlaybackProgress, PluginId, VesselError, VesselResult, domain_version,
};

use crate::library::{
    RawLibraryRow, i64_to_u64, library_status_as_str, media_type_as_str, u64_to_i64,
};
use crate::{SqliteStorage, map_sql_err};

/// Current schema version for `.backup.vessel` snapshot files.
pub const BACKUP_SCHEMA_VERSION: u32 = 1;

/// Strategy used when importing a `.backup.vessel` snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupImportMode {
    /// Keeps existing records and only overwrites when the backup entry has a newer timestamp.
    SmartMerge,
    /// Wipes existing settings, library, and watch progress before restoring the snapshot.
    ResetAndImport,
}

/// Per-plugin state and user-granted domains captured in a backup file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPluginEntry {
    pub id: PluginId,
    pub enabled: bool,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
}

/// Portable `.backup.vessel` JSON snapshot schema (`PLAN.md` 7.1.E).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VesselBackup {
    pub schema_version: u32,
    pub exported_at: String,
    pub app_version: String,
    #[serde(default)]
    pub includes_secrets: bool,
    #[serde(default)]
    pub settings: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub plugins: Vec<BackupPluginEntry>,
    #[serde(default)]
    pub library: Vec<LibraryItem>,
    #[serde(default)]
    pub watch_progress: Vec<PlaybackProgress>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,
}

impl VesselBackup {
    /// Validates basic structural invariants of a parsed backup snapshot.
    pub fn validate(&self) -> VesselResult<()> {
        if self.schema_version != BACKUP_SCHEMA_VERSION {
            return Err(VesselError::InvalidInput(format!(
                "unsupported backup schemaVersion: {}",
                self.schema_version
            )));
        }
        Ok(())
    }
}

struct RawBackupProgressRow {
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
}

impl RawBackupProgressRow {
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
        })
    }

    fn into_domain(self) -> VesselResult<PlaybackProgress> {
        let plugin_id = PluginId::new(self.plugin_id)?;
        let media = MediaRef::new(plugin_id, self.native_id)?;
        Ok(PlaybackProgress {
            media,
            stream_id: self.stream_id,
            canonical_id: self.canonical_id,
            season: self.season,
            episode: self.episode,
            absolute_episode: self.absolute_episode,
            position_ms: i64_to_u64(self.position_ms, "position_ms")?,
            duration_ms: i64_to_u64(self.duration_ms, "duration_ms")?,
            completed: self.completed != 0,
            last_watched_at: self.last_watched_at,
        })
    }
}

fn export_settings(conn: &Connection) -> VesselResult<BTreeMap<String, serde_json::Value>> {
    let mut stmt = conn
        .prepare("SELECT key, value_json FROM settings ORDER BY key ASC")
        .map_err(map_sql_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(map_sql_err)?;

    let mut settings = BTreeMap::new();
    for row in rows {
        let (key, value_json) = row.map_err(map_sql_err)?;
        let parsed: serde_json::Value = serde_json::from_str(&value_json)
            .map_err(|err| VesselError::Storage(format!("decode setting '{key}': {err}")))?;
        settings.insert(key, parsed);
    }
    Ok(settings)
}

fn export_plugins(conn: &Connection) -> VesselResult<Vec<BackupPluginEntry>> {
    let mut stmt = conn
        .prepare("SELECT id, enabled FROM plugins ORDER BY id ASC")
        .map_err(map_sql_err)?;
    let plugin_rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(map_sql_err)?;

    let mut entries = Vec::new();
    let mut domain_stmt = conn
        .prepare(
            "SELECT domain FROM plugin_allowed_domains WHERE plugin_id = ?1 ORDER BY domain ASC",
        )
        .map_err(map_sql_err)?;

    for row in plugin_rows {
        let (id_raw, enabled) = row.map_err(map_sql_err)?;
        let domain_rows = domain_stmt
            .query_map([&id_raw], |r| r.get::<_, String>(0))
            .map_err(map_sql_err)?;
        let mut allowed_domains = Vec::new();
        for domain in domain_rows {
            allowed_domains.push(domain.map_err(map_sql_err)?);
        }
        entries.push(BackupPluginEntry {
            id: PluginId::new(id_raw)?,
            enabled: enabled != 0,
            allowed_domains,
        });
    }
    Ok(entries)
}

fn export_library(conn: &Connection) -> VesselResult<Vec<LibraryItem>> {
    let mut stmt = conn
        .prepare(
            "SELECT plugin_id, native_id, canonical_id, media_type, is_anime,
                    title, poster_url, year, status, added_at, updated_at
             FROM library_items
             ORDER BY plugin_id ASC, native_id ASC",
        )
        .map_err(map_sql_err)?;
    let rows = stmt
        .query_map([], RawLibraryRow::from_sql)
        .map_err(map_sql_err)?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(map_sql_err)?.into_domain()?);
    }
    Ok(items)
}

fn export_watch_progress(conn: &Connection) -> VesselResult<Vec<PlaybackProgress>> {
    let mut stmt = conn
        .prepare(
            "SELECT plugin_id, native_id, stream_id, canonical_id, season, episode,
                    absolute_episode, position_ms, duration_ms, completed, last_watched_at
             FROM watch_progress
             ORDER BY plugin_id ASC, native_id ASC, season ASC, episode ASC",
        )
        .map_err(map_sql_err)?;
    let rows = stmt
        .query_map([], RawBackupProgressRow::from_sql)
        .map_err(map_sql_err)?;

    let mut progress = Vec::new();
    for row in rows {
        progress.push(row.map_err(map_sql_err)?.into_domain()?);
    }
    Ok(progress)
}

fn export_secrets(conn: &Connection) -> VesselResult<BTreeMap<String, String>> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM secrets ORDER BY key ASC")
        .map_err(map_sql_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(map_sql_err)?;

    let mut secrets = BTreeMap::new();
    for row in rows {
        let (key, val) = row.map_err(map_sql_err)?;
        secrets.insert(key, val);
    }
    Ok(secrets)
}

fn import_settings_and_secrets(tx: &Transaction<'_>, backup: &VesselBackup) -> VesselResult<()> {
    for (key, val) in &backup.settings {
        let value_json = serde_json::to_string(val)
            .map_err(|err| VesselError::Storage(format!("encode setting '{key}': {err}")))?;
        tx.execute(
            "INSERT INTO settings (key, value_json)
             VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
            (key, &value_json),
        )
        .map_err(map_sql_err)?;
    }

    if backup.includes_secrets {
        for (key, val) in &backup.secrets {
            tx.execute(
                "INSERT INTO secrets (key, value, updated_at)
                 VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(key) DO UPDATE SET
                     value = excluded.value,
                     updated_at = excluded.updated_at",
                (key, val),
            )
            .map_err(map_sql_err)?;
        }
    }
    Ok(())
}

fn import_plugins(tx: &Transaction<'_>, plugins: &[BackupPluginEntry]) -> VesselResult<()> {
    for entry in plugins {
        let id = entry.id.as_str();
        let exists: Option<i64> = tx
            .query_row("SELECT 1 FROM plugins WHERE id = ?1", [id], |r| r.get(0))
            .optional()
            .map_err(map_sql_err)?;
        if exists.is_none() {
            continue;
        }

        tx.execute(
            "UPDATE plugins SET enabled = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?2",
            (i32::from(entry.enabled), id),
        )
        .map_err(map_sql_err)?;

        for raw_domain in &entry.allowed_domains {
            let domain = raw_domain.trim().to_ascii_lowercase();
            if domain.is_empty() {
                continue;
            }
            tx.execute(
                "INSERT INTO plugin_allowed_domains (plugin_id, domain, grant_source, created_at)
                 VALUES (?1, ?2, 'user_permanent', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(plugin_id, domain) DO UPDATE SET grant_source = excluded.grant_source",
                (id, &domain),
            )
            .map_err(map_sql_err)?;
        }
    }
    Ok(())
}

fn import_library(
    tx: &Transaction<'_>,
    library: &[LibraryItem],
    mode: BackupImportMode,
) -> VesselResult<()> {
    for item in library {
        if mode == BackupImportMode::SmartMerge {
            let existing_updated_at: Option<String> = tx
                .query_row(
                    "SELECT updated_at FROM library_items WHERE plugin_id = ?1 AND native_id = ?2",
                    (item.media.plugin_id.as_str(), &item.media.native_id),
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_sql_err)?;
            if let Some(existing) = existing_updated_at
                && existing.as_str() > item.updated_at.as_str()
            {
                continue;
            }
        }

        let row_id = format!("{}:{}", item.media.plugin_id, item.media.native_id);
        tx.execute(
            "INSERT INTO library_items (
                 id, plugin_id, native_id, canonical_id, media_type, is_anime,
                 title, poster_url, year, status, added_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(plugin_id, native_id) DO UPDATE SET
                 canonical_id = excluded.canonical_id,
                 media_type = excluded.media_type,
                 is_anime = excluded.is_anime,
                 title = excluded.title,
                 poster_url = excluded.poster_url,
                 year = excluded.year,
                 status = excluded.status,
                 updated_at = excluded.updated_at",
            (
                &row_id,
                item.media.plugin_id.as_str(),
                &item.media.native_id,
                &item.canonical_id,
                media_type_as_str(item.media_type),
                i32::from(item.is_anime),
                &item.title,
                &item.poster_url,
                item.year,
                library_status_as_str(item.status),
                &item.added_at,
                &item.updated_at,
            ),
        )
        .map_err(map_sql_err)?;
    }
    Ok(())
}

fn import_watch_progress(
    tx: &Transaction<'_>,
    progress: &[PlaybackProgress],
    mode: BackupImportMode,
) -> VesselResult<()> {
    for prog in progress {
        let position_ms = u64_to_i64(prog.position_ms, "position_ms")?;
        let duration_ms = u64_to_i64(prog.duration_ms, "duration_ms")?;

        if mode == BackupImportMode::SmartMerge {
            let existing_ts: Option<String> = tx
                .query_row(
                    "SELECT last_watched_at FROM watch_progress
                     WHERE plugin_id = ?1 AND native_id = ?2 AND season = ?3 AND episode = ?4",
                    (
                        prog.media.plugin_id.as_str(),
                        &prog.media.native_id,
                        prog.season,
                        prog.episode,
                    ),
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_sql_err)?;
            if let Some(existing) = existing_ts
                && existing.as_str() > prog.last_watched_at.as_str()
            {
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
                 absolute_episode, position_ms, duration_ms, completed, last_watched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(plugin_id, native_id, season, episode) DO UPDATE SET
                 stream_id = excluded.stream_id,
                 canonical_id = excluded.canonical_id,
                 absolute_episode = excluded.absolute_episode,
                 position_ms = excluded.position_ms,
                 duration_ms = excluded.duration_ms,
                 completed = excluded.completed,
                 last_watched_at = excluded.last_watched_at",
            (
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
            ),
        )
        .map_err(map_sql_err)?;
    }
    Ok(())
}

impl SqliteStorage {
    /// Captures a consistent `.backup.vessel` snapshot (secrets omitted unless `include_secrets` is `true`).
    pub async fn export_backup(&self, include_secrets: bool) -> VesselResult<VesselBackup> {
        self.actor()
            .execute(move |db| {
                let conn = db.conn();
                let exported_at: String = conn
                    .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ', 'now')", [], |r| {
                        r.get(0)
                    })
                    .map_err(map_sql_err)?;

                let secrets = if include_secrets {
                    export_secrets(conn)?
                } else {
                    BTreeMap::new()
                };

                Ok(VesselBackup {
                    schema_version: BACKUP_SCHEMA_VERSION,
                    exported_at,
                    app_version: domain_version().to_owned(),
                    includes_secrets: include_secrets,
                    settings: export_settings(conn)?,
                    plugins: export_plugins(conn)?,
                    library: export_library(conn)?,
                    watch_progress: export_watch_progress(conn)?,
                    secrets,
                })
            })
            .await
    }

    /// Exports a pretty-printed `.backup.vessel` JSON string.
    pub async fn export_backup_json(&self, include_secrets: bool) -> VesselResult<String> {
        let backup = self.export_backup(include_secrets).await?;
        serde_json::to_string_pretty(&backup)
            .map_err(|err| VesselError::Storage(format!("encode backup json: {err}")))
    }

    /// Restores a `.backup.vessel` snapshot atomically inside a single SQLite transaction.
    pub async fn import_backup(
        &self,
        backup: VesselBackup,
        mode: BackupImportMode,
    ) -> VesselResult<()> {
        backup.validate()?;

        self.actor()
            .execute(move |db| {
                let tx = db.conn_mut().transaction().map_err(map_sql_err)?;

                if mode == BackupImportMode::ResetAndImport {
                    tx.execute_batch(
                        "DELETE FROM watch_progress;
                         DELETE FROM library_items;
                         DELETE FROM settings;",
                    )
                    .map_err(map_sql_err)?;
                    if backup.includes_secrets {
                        tx.execute("DELETE FROM secrets", []).map_err(map_sql_err)?;
                    }
                }

                import_settings_and_secrets(&tx, &backup)?;
                import_plugins(&tx, &backup.plugins)?;
                import_library(&tx, &backup.library, mode)?;
                import_watch_progress(&tx, &backup.watch_progress, mode)?;

                tx.commit().map_err(map_sql_err)
            })
            .await
    }

    /// Parses a `.backup.vessel` JSON string and imports it using the requested mode.
    pub async fn import_backup_json(
        &self,
        raw_json: &str,
        mode: BackupImportMode,
    ) -> VesselResult<()> {
        let backup: VesselBackup = serde_json::from_str(raw_json)
            .map_err(|err| VesselError::InvalidInput(format!("invalid backup json: {err}")))?;
        self.import_backup(backup, mode).await
    }
}

#[cfg(test)]
mod tests {
    use vessel_domain::{
        LibraryItem, LibraryStatus, MediaRef, MediaType, PlaybackProgress, PluginId, SecretStore,
        StoragePort, UiPreferences,
    };

    use super::{BACKUP_SCHEMA_VERSION, BackupImportMode, VesselBackup};
    use crate::{SqliteSecretStore, SqliteStorage, StorageActor};

    fn progress_row(
        media: &MediaRef,
        episode: u32,
        position_ms: u64,
        last_watched_at: &str,
    ) -> PlaybackProgress {
        PlaybackProgress {
            media: media.clone(),
            stream_id: format!("/watch/ep-{episode}"),
            canonical_id: Some("anilist:154587".into()),
            season: 1,
            episode,
            absolute_episode: Some(episode),
            position_ms,
            duration_ms: 1_440_000,
            completed: false,
            last_watched_at: last_watched_at.into(),
        }
    }

    #[tokio::test]
    async fn smart_merge_preserves_newer_watch_progress_and_excludes_secrets_by_default() {
        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let secrets = SqliteSecretStore::new(actor.clone());
        let storage = SqliteStorage::new(actor);

        secrets
            .set("anthology.token", "top-secret")
            .await
            .expect("seed secret");

        let anthology = PluginId::new("anthology").expect("valid plugin id");
        let media = MediaRef::new(anthology, "/anime/frieren").expect("valid media ref");

        storage
            .upsert_progress(&progress_row(&media, 1, 100_000, "2026-11-18T10:00:00Z"))
            .await
            .expect("seed ep 1");
        storage
            .upsert_progress(&progress_row(&media, 2, 900_000, "2026-11-18T20:00:00Z"))
            .await
            .expect("seed ep 2");

        let snapshot = storage.export_backup(false).await.expect("export backup");
        assert!(!snapshot.includes_secrets);
        assert!(snapshot.secrets.is_empty());
        assert_eq!(snapshot.watch_progress.len(), 2);

        let incoming = VesselBackup {
            schema_version: BACKUP_SCHEMA_VERSION,
            exported_at: "2026-11-18T21:00:00Z".into(),
            app_version: "0.1.0".into(),
            includes_secrets: false,
            settings: Default::default(),
            plugins: Vec::new(),
            library: Vec::new(),
            watch_progress: vec![
                progress_row(&media, 1, 500_000, "2026-11-18T15:00:00Z"),
                progress_row(&media, 2, 300_000, "2026-11-18T12:00:00Z"),
            ],
            secrets: Default::default(),
        };

        storage
            .import_backup(incoming, BackupImportMode::SmartMerge)
            .await
            .expect("smart merge");

        let ep1 = storage
            .get_progress(&media, 1, 1)
            .await
            .expect("get ep 1")
            .expect("ep 1 exists");
        assert_eq!(ep1.position_ms, 500_000);

        let ep2 = storage
            .get_progress(&media, 1, 2)
            .await
            .expect("get ep 2")
            .expect("ep 2 exists");
        assert_eq!(ep2.position_ms, 900_000);
    }

    #[tokio::test]
    async fn reset_and_import_replaces_state_from_json_snapshot() {
        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let secrets = SqliteSecretStore::new(actor.clone());
        let storage = SqliteStorage::new(actor);

        secrets
            .set("anthology.api_key", "key-original")
            .await
            .expect("set secret");

        let mut prefs = UiPreferences::default();
        prefs.details.hide_episode_thumbnails = true;
        storage
            .set_ui_preferences(&prefs)
            .await
            .expect("save prefs");

        let anthology = PluginId::new("anthology").expect("valid plugin id");
        let media = MediaRef::new(anthology.clone(), "/anime/frieren").expect("valid media ref");
        storage
            .upsert_library_item(&LibraryItem {
                media: media.clone(),
                canonical_id: Some("anilist:154587".into()),
                media_type: MediaType::Series,
                is_anime: true,
                title: "Frieren".into(),
                poster_url: None,
                year: Some(2023),
                status: LibraryStatus::Watching,
                added_at: "2026-11-18T10:00:00Z".into(),
                updated_at: "2026-11-18T10:00:00Z".into(),
            })
            .await
            .expect("insert library item");

        let json = storage
            .export_backup_json(true)
            .await
            .expect("export backup json");

        let extra_media = MediaRef::new(anthology, "/anime/other").expect("valid extra media");
        storage
            .upsert_library_item(&LibraryItem {
                media: extra_media.clone(),
                canonical_id: None,
                media_type: MediaType::Movie,
                is_anime: true,
                title: "Extra".into(),
                poster_url: None,
                year: Some(2024),
                status: LibraryStatus::PlanToWatch,
                added_at: "2026-11-18T11:00:00Z".into(),
                updated_at: "2026-11-18T11:00:00Z".into(),
            })
            .await
            .expect("insert extra item");
        storage
            .set_ui_preferences(&UiPreferences::default())
            .await
            .expect("reset prefs");
        secrets
            .set("anthology.api_key", "key-mutated")
            .await
            .expect("mutate secret");

        storage
            .import_backup_json(&json, BackupImportMode::ResetAndImport)
            .await
            .expect("reset and import");

        let restored_prefs = storage.get_ui_preferences().await.expect("restored prefs");
        assert!(restored_prefs.details.hide_episode_thumbnails);

        let restored_library = storage.list_library(None).await.expect("restored library");
        assert_eq!(restored_library.len(), 1);
        assert_eq!(restored_library[0].media, media);

        assert_eq!(
            secrets
                .get("anthology.api_key")
                .await
                .expect("restored secret"),
            Some("key-original".into())
        );
    }
}
