//! Plugin registry, domain allowlist, and isolated plugin KV persistence.

use rusqlite::OptionalExtension;
use vessel_domain::{DomainGrantSource, PluginId, PluginManifest, VesselError, VesselResult};

use crate::{SqliteStorage, map_sql_err};

/// Persisted plugin installation record matching the `plugins` SQLite table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPlugin {
    pub manifest: PluginManifest,
    pub repo_id: Option<String>,
    pub source_type: String,
    pub source_url: Option<String>,
    pub local_path: String,
    pub sha256: Option<String>,
    pub enabled: bool,
    pub allow_dynamic_domains: bool,
    pub installed_at: String,
    pub updated_at: String,
}

fn grant_source_as_str(source: DomainGrantSource) -> &'static str {
    match source {
        DomainGrantSource::Manifest => "manifest",
        DomainGrantSource::UserPermanent => "user_permanent",
    }
}

struct RawPluginRow {
    repo_id: Option<String>,
    source_type: String,
    source_url: Option<String>,
    local_path: String,
    sha256: Option<String>,
    enabled: i64,
    allow_dynamic_domains: i64,
    manifest_json: String,
    installed_at: String,
    updated_at: String,
}

impl RawPluginRow {
    fn from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            repo_id: row.get(0)?,
            source_type: row.get(1)?,
            source_url: row.get(2)?,
            local_path: row.get(3)?,
            sha256: row.get(4)?,
            enabled: row.get(5)?,
            allow_dynamic_domains: row.get(6)?,
            manifest_json: row.get(7)?,
            installed_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }

    fn into_stored(self, id: &str) -> VesselResult<StoredPlugin> {
        let manifest: PluginManifest = serde_json::from_str(&self.manifest_json)
            .map_err(|err| VesselError::Storage(format!("decode plugin '{id}' manifest: {err}")))?;
        Ok(StoredPlugin {
            manifest,
            repo_id: self.repo_id,
            source_type: self.source_type,
            source_url: self.source_url,
            local_path: self.local_path,
            sha256: self.sha256,
            enabled: self.enabled != 0,
            allow_dynamic_domains: self.allow_dynamic_domains != 0,
            installed_at: self.installed_at,
            updated_at: self.updated_at,
        })
    }
}

impl SqliteStorage {
    /// Installs or updates a plugin row (alongside manifest-declared domains if new).
    pub async fn upsert_plugin(&self, plugin: StoredPlugin) -> VesselResult<()> {
        plugin.manifest.validate()?;
        let capabilities_json = serde_json::to_string(&plugin.manifest.capabilities)
            .map_err(|err| VesselError::Storage(format!("encode capabilities: {err}")))?;
        let languages_json = serde_json::to_string(&plugin.manifest.languages)
            .map_err(|err| VesselError::Storage(format!("encode languages: {err}")))?;
        let manifest_json = serde_json::to_string(&plugin.manifest)
            .map_err(|err| VesselError::Storage(format!("encode manifest: {err}")))?;

        self.actor()
            .execute(move |db| {
                let normalized_repo_id = plugin
                    .repo_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned);

                if let Some(ref rid) = normalized_repo_id {
                    let placeholder_src = format!("repo:{rid}");
                    db.conn()
                        .execute(
                            "INSERT OR IGNORE INTO repositories (
                                 id, source_input, resolved_url, name, is_official, updated_at
                             ) VALUES (?1, ?2, ?3, ?1, 0, ?4)",
                            (rid, &placeholder_src, &placeholder_src, &plugin.updated_at),
                        )
                        .map_err(map_sql_err)?;
                }

                let normalized_source_url = plugin
                    .source_url
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned);

                db.conn()
                    .execute(
                        "INSERT INTO plugins (
                             id, repo_id, name, version, capabilities_json, languages_json,
                             source_type, source_url, local_path, sha256, enabled, allow_dynamic_domains,
                             manifest_json, installed_at, updated_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                         ON CONFLICT(id) DO UPDATE SET
                             repo_id = excluded.repo_id,
                             name = excluded.name,
                             version = excluded.version,
                             capabilities_json = excluded.capabilities_json,
                             languages_json = excluded.languages_json,
                             source_type = excluded.source_type,
                             source_url = COALESCE(excluded.source_url, plugins.source_url),
                             local_path = excluded.local_path,
                             sha256 = excluded.sha256,
                             enabled = excluded.enabled,
                             allow_dynamic_domains = excluded.allow_dynamic_domains,
                             manifest_json = excluded.manifest_json,
                             updated_at = excluded.updated_at",
                        (
                            plugin.manifest.id.as_str(),
                            &normalized_repo_id,
                            &plugin.manifest.name,
                            &plugin.manifest.version,
                            &capabilities_json,
                            &languages_json,
                            &plugin.source_type,
                            &normalized_source_url,
                            &plugin.local_path,
                            &plugin.sha256,
                            i32::from(plugin.enabled),
                            i32::from(plugin.allow_dynamic_domains),
                            &manifest_json,
                            &plugin.installed_at,
                            &plugin.updated_at,
                        ),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Loads a single installed plugin by ID.
    pub async fn get_plugin(&self, plugin_id: &PluginId) -> VesselResult<Option<StoredPlugin>> {
        let id = plugin_id.as_str().to_owned();
        self.actor()
            .execute(move |db| {
                let row = db
                    .conn()
                    .query_row(
                        "SELECT repo_id, source_type, source_url, local_path, sha256, enabled,
                                allow_dynamic_domains, manifest_json, installed_at, updated_at
                         FROM plugins WHERE id = ?1",
                        [&id],
                        RawPluginRow::from_sql,
                    )
                    .optional()
                    .map_err(map_sql_err)?;

                row.map(|raw| raw.into_stored(&id)).transpose()
            })
            .await
    }

    /// Lists all installed plugins ordered by name.
    pub async fn list_plugins(&self) -> VesselResult<Vec<StoredPlugin>> {
        self.actor()
            .execute(|db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT id, repo_id, source_type, source_url, local_path, sha256, enabled,
                                allow_dynamic_domains, manifest_json, installed_at, updated_at
                         FROM plugins ORDER BY name ASC",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([], |row| {
                        let id: String = row.get(0)?;
                        let raw = RawPluginRow {
                            repo_id: row.get(1)?,
                            source_type: row.get(2)?,
                            source_url: row.get(3)?,
                            local_path: row.get(4)?,
                            sha256: row.get(5)?,
                            enabled: row.get(6)?,
                            allow_dynamic_domains: row.get(7)?,
                            manifest_json: row.get(8)?,
                            installed_at: row.get(9)?,
                            updated_at: row.get(10)?,
                        };
                        Ok((id, raw))
                    })
                    .map_err(map_sql_err)?;

                let mut plugins = Vec::new();
                for entry in rows {
                    let (id, raw) = entry.map_err(map_sql_err)?;
                    plugins.push(raw.into_stored(&id)?);
                }
                Ok(plugins)
            })
            .await
    }

    /// Enables or disables an installed plugin.
    pub async fn set_plugin_enabled(
        &self,
        plugin_id: &PluginId,
        enabled: bool,
    ) -> VesselResult<()> {
        let id = plugin_id.as_str().to_owned();
        self.actor()
            .execute(move |db| {
                let changed = db
                    .conn()
                    .execute(
                        "UPDATE plugins SET enabled = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                         WHERE id = ?2",
                        (i32::from(enabled), &id),
                    )
                    .map_err(map_sql_err)?;
                if changed == 0 {
                    return Err(VesselError::NotFound(format!("plugin '{id}'")));
                }
                Ok(())
            })
            .await
    }

    /// Removes a plugin and cleans up orphaned library, progress, override, and metadata cache entries.
    pub async fn remove_plugin(&self, plugin_id: &PluginId) -> VesselResult<()> {
        let id = plugin_id.as_str().to_owned();
        self.actor()
            .execute(move |db| {
                let tx = db.conn_mut().transaction().map_err(map_sql_err)?;
                tx.execute("DELETE FROM library_items WHERE plugin_id = ?1", [&id])
                    .map_err(map_sql_err)?;
                tx.execute("DELETE FROM watch_progress WHERE plugin_id = ?1", [&id])
                    .map_err(map_sql_err)?;
                tx.execute("DELETE FROM metadata_overrides WHERE plugin_id = ?1", [&id])
                    .map_err(map_sql_err)?;
                let meta3_pattern = format!("meta3:{id}:%");
                let meta2_pattern = format!("meta2:{id}:%");
                let meta_pattern = format!("meta:{id}:%");
                let detail_pattern = format!("detail:{id}:%");
                tx.execute(
                    "DELETE FROM metadata_cache WHERE cache_key LIKE ?1 OR cache_key LIKE ?2 OR cache_key LIKE ?3 OR cache_key LIKE ?4",
                    (&meta3_pattern, &meta2_pattern, &meta_pattern, &detail_pattern),
                )
                .map_err(map_sql_err)?;
                tx.execute("DELETE FROM plugins WHERE id = ?1", [&id])
                    .map_err(map_sql_err)?;
                tx.commit().map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Checks whether a normalized domain is present in `plugin_allowed_domains`.
    pub async fn is_domain_allowed(
        &self,
        plugin_id: &PluginId,
        domain: &str,
    ) -> VesselResult<bool> {
        let id = plugin_id.as_str().to_owned();
        let domain = domain.trim().to_ascii_lowercase();
        if domain.is_empty() {
            return Err(VesselError::InvalidInput("domain cannot be empty".into()));
        }

        self.actor()
            .execute(move |db| {
                let found: Option<i64> = db
                    .conn()
                    .query_row(
                        "SELECT 1 FROM plugin_allowed_domains WHERE plugin_id = ?1 AND domain = ?2",
                        (&id, &domain),
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(map_sql_err)?;
                Ok(found.is_some())
            })
            .await
    }

    /// Grants permanent or manifest-declared access to a domain for `plugin_id`.
    pub async fn grant_domain(
        &self,
        plugin_id: &PluginId,
        domain: &str,
        source: DomainGrantSource,
    ) -> VesselResult<()> {
        let id = plugin_id.as_str().to_owned();
        let domain = domain.trim().to_ascii_lowercase();
        if domain.is_empty() {
            return Err(VesselError::InvalidInput("domain cannot be empty".into()));
        }
        let source_str = grant_source_as_str(source);

        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO plugin_allowed_domains (plugin_id, domain, grant_source, created_at)
                         VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                         ON CONFLICT(plugin_id, domain) DO UPDATE SET grant_source = excluded.grant_source",
                        (&id, &domain, source_str),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Lists all persisted allowed domains for `plugin_id`.
    pub async fn list_allowed_domains(&self, plugin_id: &PluginId) -> VesselResult<Vec<String>> {
        let id = plugin_id.as_str().to_owned();
        self.actor()
            .execute(move |db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT domain FROM plugin_allowed_domains WHERE plugin_id = ?1 ORDER BY domain ASC",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([&id], |row| row.get(0))
                    .map_err(map_sql_err)?;
                let mut domains = Vec::new();
                for item in rows {
                    domains.push(item.map_err(map_sql_err)?);
                }
                Ok(domains)
            })
            .await
    }

    /// Reads an isolated per-plugin KV entry from `plugin_kv`.
    pub async fn get_plugin_kv(
        &self,
        plugin_id: &PluginId,
        key: &str,
    ) -> VesselResult<Option<String>> {
        let id = plugin_id.as_str().to_owned();
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(VesselError::InvalidInput(
                "plugin kv key cannot be empty".into(),
            ));
        }

        self.actor()
            .execute(move |db| {
                db.conn()
                    .query_row(
                        "SELECT value FROM plugin_kv WHERE plugin_id = ?1 AND key = ?2",
                        (&id, &key),
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(map_sql_err)
            })
            .await
    }

    /// Writes or updates an isolated per-plugin KV entry in `plugin_kv`.
    pub async fn set_plugin_kv(
        &self,
        plugin_id: &PluginId,
        key: &str,
        value: &str,
    ) -> VesselResult<()> {
        let id = plugin_id.as_str().to_owned();
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(VesselError::InvalidInput(
                "plugin kv key cannot be empty".into(),
            ));
        }
        let value = value.to_owned();

        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO plugin_kv (plugin_id, key, value, updated_at)
                         VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                         ON CONFLICT(plugin_id, key) DO UPDATE SET
                             value = excluded.value,
                             updated_at = excluded.updated_at",
                        (&id, &key, &value),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Deletes a single isolated per-plugin KV entry.
    pub async fn remove_plugin_kv(&self, plugin_id: &PluginId, key: &str) -> VesselResult<()> {
        let id = plugin_id.as_str().to_owned();
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(VesselError::InvalidInput(
                "plugin kv key cannot be empty".into(),
            ));
        }

        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "DELETE FROM plugin_kv WHERE plugin_id = ?1 AND key = ?2",
                        (&id, &key),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use vessel_domain::{
        ContentCategory, DomainGrantSource, PluginCapability, PluginId, PluginManifest,
    };

    use super::StoredPlugin;
    use crate::{SqliteStorage, StorageActor, map_sql_err};

    fn sample_plugin(id: &str) -> StoredPlugin {
        StoredPlugin {
            manifest: PluginManifest {
                schema_version: 1,
                id: PluginId::new(id).expect("valid plugin id"),
                name: "Anthology".into(),
                version: "1.0.0".into(),
                author: Some("falsisdev".into()),
                description: None,
                entry: "index.ts".into(),
                icon: None,
                min_core_version: None,
                capabilities: vec![PluginCapability::Vod, PluginCapability::Search],
                content_types: vec![ContentCategory::Anime],
                languages: vec!["tr".into()],
                domains: vec!["anthology.example".into()],
                allow_any_https: false,
                is_adult: false,
                files: Vec::new(),
                settings_schema: Vec::new(),
            },
            repo_id: None,
            source_type: "local_dev".into(),
            source_url: None,
            local_path: "/tmp/anthology".into(),
            sha256: None,
            enabled: true,
            allow_dynamic_domains: false,
            installed_at: "2026-11-18T17:00:00Z".into(),
            updated_at: "2026-11-18T17:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn manages_plugins_domains_kv_and_cascades_deletes() {
        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let storage = SqliteStorage::new(actor);

        let mut plugin = sample_plugin("anthology");
        plugin.repo_id = Some("Community Nuvio Repo".into());
        let plugin_id = plugin.manifest.id.clone();

        storage.upsert_plugin(plugin.clone()).await.expect("upsert");
        assert_eq!(
            storage.get_plugin(&plugin_id).await.expect("get"),
            Some(plugin)
        );

        storage
            .set_plugin_enabled(&plugin_id, false)
            .await
            .expect("disable plugin");
        assert!(
            !storage
                .get_plugin(&plugin_id)
                .await
                .expect("get disabled")
                .expect("exists")
                .enabled
        );

        storage
            .grant_domain(
                &plugin_id,
                "CDN.Anthology.Example",
                DomainGrantSource::UserPermanent,
            )
            .await
            .expect("grant domain");
        assert!(
            storage
                .is_domain_allowed(&plugin_id, "cdn.anthology.example")
                .await
                .expect("check allowed domain")
        );

        storage
            .set_plugin_kv(&plugin_id, "mirror", "https://mirror.anthology.example")
            .await
            .expect("set kv");
        assert_eq!(
            storage
                .get_plugin_kv(&plugin_id, "mirror")
                .await
                .expect("get kv"),
            Some("https://mirror.anthology.example".into())
        );

        storage
            .remove_plugin(&plugin_id)
            .await
            .expect("remove plugin");
        assert_eq!(storage.get_plugin(&plugin_id).await.expect("gone"), None);
        assert!(
            !storage
                .is_domain_allowed(&plugin_id, "cdn.anthology.example")
                .await
                .expect("domain cascaded")
        );
        let kv_rows: i64 = storage
            .actor()
            .execute(|db| {
                db.conn()
                    .query_row("SELECT COUNT(*) FROM plugin_kv", [], |row| row.get(0))
                    .map_err(map_sql_err)
            })
            .await
            .expect("count plugin_kv");
        assert_eq!(kv_rows, 0);
    }
}
