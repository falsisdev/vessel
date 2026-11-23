//! High-level typed storage repository implementing `StoragePort`.

use async_trait::async_trait;
use rusqlite::OptionalExtension;
use serde::Serialize;
use serde::de::DeserializeOwned;
use vessel_domain::{
    DomainGrantSource, LibraryItem, LibraryStatus, MediaRef, PlaybackProgress, PluginId,
    StoragePort, UiPreferences, VesselError, VesselResult,
};

use crate::{StorageActor, map_sql_err};

/// Canonical settings key for modular `UiPreferences`.
pub const UI_PREFERENCES_KEY: &str = "ui_preferences";

/// Main persistent repository delegating serialized operations to `StorageActor`.
#[derive(Debug, Clone)]
pub struct SqliteStorage {
    actor: StorageActor,
}

impl SqliteStorage {
    /// Wraps an initialized `StorageActor` handle.
    pub fn new(actor: StorageActor) -> Self {
        Self { actor }
    }

    /// Borrows the underlying `StorageActor`.
    pub fn actor(&self) -> &StorageActor {
        &self.actor
    }

    /// Reads a raw JSON setting string from the `settings` table.
    pub async fn get_setting_raw(&self, key: &str) -> VesselResult<Option<String>> {
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(VesselError::InvalidInput(
                "setting key cannot be empty".into(),
            ));
        }

        self.actor
            .execute(move |db| {
                db.conn()
                    .query_row(
                        "SELECT value_json FROM settings WHERE key = ?1",
                        [&key],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(map_sql_err)
            })
            .await
    }

    /// Upserts a raw JSON setting string in the `settings` table.
    pub async fn set_setting_raw(&self, key: &str, value_json: &str) -> VesselResult<()> {
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(VesselError::InvalidInput(
                "setting key cannot be empty".into(),
            ));
        }
        let value_json = value_json.to_owned();

        self.actor
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO settings (key, value_json)
                         VALUES (?1, ?2)
                         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
                        (&key, &value_json),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Reads and deserializes a typed JSON setting into an owned `T`.
    pub async fn get_json_setting<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> VesselResult<Option<T>> {
        let Some(raw) = self.get_setting_raw(key).await? else {
            return Ok(None);
        };
        let parsed = serde_json::from_str(&raw)
            .map_err(|err| VesselError::Storage(format!("decode json setting '{key}': {err}")))?;
        Ok(Some(parsed))
    }

    /// Serializes `value` as JSON and upserts it under `key`.
    pub async fn set_json_setting<T: Serialize>(&self, key: &str, value: &T) -> VesselResult<()> {
        let json = serde_json::to_string(value)
            .map_err(|err| VesselError::Storage(format!("encode json setting '{key}': {err}")))?;
        self.set_setting_raw(key, &json).await
    }

    /// Loads `UiPreferences`, returning defaults when not yet persisted.
    pub async fn get_ui_preferences(&self) -> VesselResult<UiPreferences> {
        Ok(self
            .get_json_setting(UI_PREFERENCES_KEY)
            .await?
            .unwrap_or_default())
    }

    /// Persists `UiPreferences` under `UI_PREFERENCES_KEY`.
    pub async fn set_ui_preferences(&self, prefs: &UiPreferences) -> VesselResult<()> {
        self.set_json_setting(UI_PREFERENCES_KEY, prefs).await
    }

    /// Loads all persisted `ts_compile_cache` `(source_hash, compiled_js)` pairs.
    pub async fn load_ts_compile_cache(&self) -> VesselResult<Vec<(String, String)>> {
        self.actor
            .execute(|db| {
                let mut stmt = db
                    .conn()
                    .prepare("SELECT source_hash, compiled_js FROM ts_compile_cache")
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(map_sql_err)?;
                let mut entries = Vec::new();
                for item in rows {
                    entries.push(item.map_err(map_sql_err)?);
                }
                Ok(entries)
            })
            .await
    }

    /// Stores a compiled TypeScript output in `ts_compile_cache`.
    pub async fn upsert_ts_compile_cache(
        &self,
        source_hash: &str,
        compiled_js: &str,
    ) -> VesselResult<()> {
        let hash = source_hash.to_owned();
        let js = compiled_js.to_owned();
        self.actor
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO ts_compile_cache (source_hash, compiled_js, updated_at)
                         VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                         ON CONFLICT(source_hash) DO UPDATE SET
                             compiled_js = excluded.compiled_js,
                             updated_at = excluded.updated_at",
                        (&hash, &js),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }
}

#[async_trait]
impl StoragePort for SqliteStorage {
    async fn get_setting(&self, key: &str) -> VesselResult<Option<String>> {
        self.get_setting_raw(key).await
    }

    async fn set_setting(&self, key: &str, value_json: &str) -> VesselResult<()> {
        self.set_setting_raw(key, value_json).await
    }

    async fn get_plugin_kv(&self, plugin_id: &PluginId, key: &str) -> VesselResult<Option<String>> {
        SqliteStorage::get_plugin_kv(self, plugin_id, key).await
    }

    async fn set_plugin_kv(
        &self,
        plugin_id: &PluginId,
        key: &str,
        value: &str,
    ) -> VesselResult<()> {
        SqliteStorage::set_plugin_kv(self, plugin_id, key, value).await
    }

    async fn remove_plugin_kv(&self, plugin_id: &PluginId, key: &str) -> VesselResult<()> {
        SqliteStorage::remove_plugin_kv(self, plugin_id, key).await
    }

    async fn is_domain_allowed(&self, plugin_id: &PluginId, domain: &str) -> VesselResult<bool> {
        SqliteStorage::is_domain_allowed(self, plugin_id, domain).await
    }

    async fn grant_domain(
        &self,
        plugin_id: &PluginId,
        domain: &str,
        source: DomainGrantSource,
    ) -> VesselResult<()> {
        SqliteStorage::grant_domain(self, plugin_id, domain, source).await
    }

    async fn get_progress(
        &self,
        media: &MediaRef,
        season: u32,
        episode: u32,
    ) -> VesselResult<Option<PlaybackProgress>> {
        self.get_watch_progress(media, season, episode).await
    }

    async fn upsert_progress(&self, progress: &PlaybackProgress) -> VesselResult<()> {
        self.upsert_watch_progress(progress).await
    }

    async fn list_library(&self, status: Option<LibraryStatus>) -> VesselResult<Vec<LibraryItem>> {
        self.list_library_items(status).await
    }

    async fn upsert_library_item(&self, item: &LibraryItem) -> VesselResult<()> {
        self.upsert_library(item).await
    }

    async fn remove_library_item(&self, media: &MediaRef) -> VesselResult<()> {
        self.delete_library_item(media).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde::{Deserialize, Serialize};
    use vessel_domain::{
        LibraryItem, LibraryStatus, MediaRef, MediaType, NavLayout, PlayClickBehavior,
        PlaybackProgress, PluginId, StoragePort, UiPreferences, VesselError,
    };

    use super::{SqliteStorage, UI_PREFERENCES_KEY};
    use crate::StorageActor;

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct AppConfig {
        preferred_language: String,
        always_proxy: bool,
    }

    #[tokio::test]
    async fn round_trips_json_settings_and_ui_preferences() {
        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let storage = SqliteStorage::new(actor);

        let defaults = storage.get_ui_preferences().await.expect("default prefs");
        assert_eq!(defaults, UiPreferences::default());

        let mut custom = UiPreferences::default();
        custom.shell.layout = NavLayout::TopNavbar;
        custom.playback.click_behavior = PlayClickBehavior::AlwaysAskSource;
        custom.details.hide_episode_thumbnails = true;

        storage
            .set_ui_preferences(&custom)
            .await
            .expect("save ui preferences");

        let loaded = storage.get_ui_preferences().await.expect("load prefs");
        assert_eq!(loaded, custom);

        let cfg = AppConfig {
            preferred_language: "tr".into(),
            always_proxy: true,
        };
        storage
            .set_json_setting("app_settings", &cfg)
            .await
            .expect("save app settings");
        assert_eq!(
            storage
                .get_json_setting::<AppConfig>("app_settings")
                .await
                .expect("load app settings"),
            Some(cfg)
        );
    }

    #[tokio::test]
    async fn surfaces_corrupt_json_setting_as_storage_error() {
        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let storage = SqliteStorage::new(actor);

        storage
            .set_setting_raw(UI_PREFERENCES_KEY, "{not-valid-json")
            .await
            .expect("write raw corrupt json");

        let res = storage.get_ui_preferences().await;
        assert!(matches!(res, Err(VesselError::Storage(_))));
    }

    #[tokio::test]
    async fn implements_storage_port_for_library_and_progress() {
        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let port: Arc<dyn StoragePort> = Arc::new(SqliteStorage::new(actor));

        let anthology = PluginId::new("anthology").expect("valid plugin id");
        let gintama = MediaRef::new(anthology, "/anime/gintama").expect("valid media ref");

        let mut item = LibraryItem {
            media: gintama.clone(),
            canonical_id: Some("anilist:918".into()),
            media_type: MediaType::Series,
            is_anime: true,
            title: "Gintama".into(),
            poster_url: None,
            year: Some(2006),
            status: LibraryStatus::Watching,
            added_at: "2026-11-18T17:00:00Z".into(),
            updated_at: "2026-11-18T17:00:00Z".into(),
        };
        port.upsert_library_item(&item)
            .await
            .expect("insert library item");

        item.status = LibraryStatus::Completed;
        item.updated_at = "2026-11-18T18:00:00Z".into();
        port.upsert_library_item(&item)
            .await
            .expect("update library item");

        assert!(
            port.list_library(Some(LibraryStatus::Watching))
                .await
                .expect("list watching")
                .is_empty()
        );
        let completed = port
            .list_library(Some(LibraryStatus::Completed))
            .await
            .expect("list completed");
        assert_eq!(completed.len(), 1);
        assert!(completed[0].is_anime);
        assert_eq!(completed[0].added_at, "2026-11-18T17:00:00Z");

        let mut progress = PlaybackProgress {
            media: gintama.clone(),
            stream_id: "/watch/gintama-50".into(),
            canonical_id: Some("anilist:918".into()),
            season: 1,
            episode: 50,
            absolute_episode: Some(50),
            position_ms: 420_000,
            duration_ms: 1_440_000,
            completed: false,
            last_watched_at: "2026-11-18T17:30:00Z".into(),
        };
        port.upsert_progress(&progress)
            .await
            .expect("insert progress");

        progress.position_ms = 1_400_000;
        progress.completed = true;
        port.upsert_progress(&progress)
            .await
            .expect("update progress");

        let loaded = port
            .get_progress(&gintama, 1, 50)
            .await
            .expect("read progress")
            .expect("progress exists");
        assert_eq!(loaded, progress);

        port.remove_library_item(&gintama)
            .await
            .expect("delete library item");
        assert!(port.list_library(None).await.expect("list all").is_empty());
    }
}
