//! High-level typed storage repository over `StorageActor`.

use rusqlite::OptionalExtension;
use serde::Serialize;
use serde::de::DeserializeOwned;
use vessel_domain::{UiPreferences, VesselError, VesselResult};

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
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use vessel_domain::{NavLayout, PlayClickBehavior, UiPreferences, VesselError};

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
}
