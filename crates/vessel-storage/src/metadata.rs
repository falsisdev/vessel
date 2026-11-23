//! Metadata TTL cache and user manual override persistence (`metadata_cache`, `metadata_overrides`).

use rusqlite::OptionalExtension;
use vessel_domain::{MediaRef, VesselResult};

use crate::{SqliteStorage, map_sql_err};

/// User's manual metadata binding or opt-out decision for a single plugin item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataOverride {
    pub canonical_id: Option<String>,
    pub is_disabled: bool,
    pub updated_at: String,
}

impl SqliteStorage {
    /// Reads a cached metadata payload if its `expires_at` timestamp is still in the future.
    pub async fn get_metadata_cache(&self, cache_key: &str) -> VesselResult<Option<String>> {
        let key = cache_key.to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .query_row(
                        "SELECT payload_json FROM metadata_cache
                         WHERE cache_key = ?1
                           AND expires_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
                        [&key],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(map_sql_err)
            })
            .await
    }

    /// Inserts or refreshes a cached metadata JSON payload with a UTC ISO-8601 expiry timestamp.
    pub async fn upsert_metadata_cache(
        &self,
        cache_key: &str,
        payload_json: &str,
        expires_at: &str,
    ) -> VesselResult<()> {
        let key = cache_key.to_owned();
        let payload = payload_json.to_owned();
        let expiry = expires_at.to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO metadata_cache (cache_key, payload_json, expires_at)
                         VALUES (?1, ?2, ?3)
                         ON CONFLICT(cache_key) DO UPDATE SET
                             payload_json = excluded.payload_json,
                             expires_at = excluded.expires_at",
                        (&key, &payload, &expiry),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Invalidates all cached metadata entries for `(plugin_id, native_id)` across languages.
    pub async fn invalidate_metadata_cache(
        &self,
        plugin_id: &vessel_domain::PluginId,
        native_id: &str,
    ) -> VesselResult<()> {
        let legacy_prefix = format!("meta:{}:{native_id}:%", plugin_id.as_str());
        let v2_prefix = format!("meta2:{}:{native_id}:%", plugin_id.as_str());
        let current_prefix = format!("meta3:{}:{native_id}:%", plugin_id.as_str());
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "DELETE FROM metadata_cache WHERE cache_key LIKE ?1 OR cache_key LIKE ?2 OR cache_key LIKE ?3",
                        (&legacy_prefix, &v2_prefix, &current_prefix),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Reads any user manual override or disable flag for `media`.
    pub async fn get_metadata_override(
        &self,
        media: &MediaRef,
    ) -> VesselResult<Option<MetadataOverride>> {
        let plugin_id = media.plugin_id.as_str().to_owned();
        let native_id = media.native_id.clone();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .query_row(
                        "SELECT canonical_id, is_disabled, updated_at
                         FROM metadata_overrides
                         WHERE plugin_id = ?1 AND native_id = ?2",
                        (&plugin_id, &native_id),
                        |row| {
                            let canonical_id: Option<String> = row.get(0)?;
                            let is_disabled: i64 = row.get(1)?;
                            let updated_at: String = row.get(2)?;
                            Ok(MetadataOverride {
                                canonical_id,
                                is_disabled: is_disabled != 0,
                                updated_at,
                            })
                        },
                    )
                    .optional()
                    .map_err(map_sql_err)
            })
            .await
    }

    /// Persists a manual canonical match or disables external enrichment for `media`.
    pub async fn set_metadata_override(
        &self,
        media: &MediaRef,
        canonical_id: Option<&str>,
        is_disabled: bool,
    ) -> VesselResult<()> {
        let plugin_id = media.plugin_id.as_str().to_owned();
        let native_id = media.native_id.clone();
        let canonical = canonical_id.map(str::to_owned);
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO metadata_overrides (plugin_id, native_id, canonical_id, is_disabled, updated_at)
                         VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                         ON CONFLICT(plugin_id, native_id) DO UPDATE SET
                             canonical_id = excluded.canonical_id,
                             is_disabled = excluded.is_disabled,
                             updated_at = excluded.updated_at",
                        (&plugin_id, &native_id, &canonical, i32::from(is_disabled)),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }
}
