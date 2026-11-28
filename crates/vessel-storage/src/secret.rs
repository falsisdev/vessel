//! SQLite-backed `SecretStore` implementation and cross-platform migration wrapper.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use rusqlite::OptionalExtension;
use tokio::sync::Mutex;
use vessel_domain::{SecretStore, VesselError, VesselResult};

use crate::{StorageActor, map_sql_err};

/// Persists sensitive values in the SQLite `secrets` table via `StorageActor`.
///
/// Used as a legacy migration source and headless fallback when an OS keyring is unavailable.
#[derive(Debug, Clone)]
pub struct SqliteSecretStore {
    actor: StorageActor,
}

impl SqliteSecretStore {
    /// Creates a `SqliteSecretStore` backed by the provided `StorageActor`.
    pub fn new(actor: StorageActor) -> Self {
        Self { actor }
    }
}

#[async_trait]
impl SecretStore for SqliteSecretStore {
    async fn get(&self, key: &str) -> VesselResult<Option<String>> {
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(VesselError::InvalidInput(
                "secret key cannot be empty".into(),
            ));
        }

        self.actor
            .execute(move |db| {
                db.conn()
                    .query_row("SELECT value FROM secrets WHERE key = ?1", [&key], |row| {
                        row.get(0)
                    })
                    .optional()
                    .map_err(map_sql_err)
            })
            .await
    }

    async fn set(&self, key: &str, value: &str) -> VesselResult<()> {
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(VesselError::InvalidInput(
                "secret key cannot be empty".into(),
            ));
        }
        let value = value.to_owned();

        self.actor
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO secrets (key, value, updated_at)
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

    async fn delete(&self, key: &str) -> VesselResult<()> {
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(VesselError::InvalidInput(
                "secret key cannot be empty".into(),
            ));
        }

        self.actor
            .execute(move |db| {
                db.conn()
                    .execute("DELETE FROM secrets WHERE key = ?1", [&key])
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    async fn list_keys(&self) -> VesselResult<Vec<String>> {
        self.actor
            .execute(|db| {
                let mut stmt = db
                    .conn()
                    .prepare("SELECT key FROM secrets ORDER BY key ASC")
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(map_sql_err)?;
                let mut keys = Vec::new();
                for row in rows {
                    keys.push(row.map_err(map_sql_err)?);
                }
                Ok(keys)
            })
            .await
    }
}

/// Wraps a primary hardware/OS-backed `SecretStore` and performs a one-time migration
/// of any legacy rows stored in the SQLite `secrets` table, deleting migrated SQLite rows.
pub struct MigratingSecretStore {
    primary: Arc<dyn SecretStore>,
    legacy: SqliteSecretStore,
    migration_lock: Mutex<()>,
    migrated: AtomicBool,
}

impl std::fmt::Debug for MigratingSecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MigratingSecretStore")
            .field("migrated", &self.migrated.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl MigratingSecretStore {
    /// Creates a `MigratingSecretStore` targeting `primary` and draining `legacy`.
    pub fn new(primary: Arc<dyn SecretStore>, legacy: SqliteSecretStore) -> Self {
        Self {
            primary,
            legacy,
            migration_lock: Mutex::new(()),
            migrated: AtomicBool::new(false),
        }
    }

    /// Moves all remaining plaintext rows from SQLite `secrets` into `primary` and deletes them from SQLite.
    pub async fn migrate_legacy_secrets(&self) -> VesselResult<usize> {
        if self.migrated.load(Ordering::Acquire) {
            return Ok(0);
        }
        let _guard = self.migration_lock.lock().await;
        if self.migrated.load(Ordering::Acquire) {
            return Ok(0);
        }

        let legacy_keys = self.legacy.list_keys().await?;
        let mut count = 0usize;
        for key in legacy_keys {
            if let Some(value) = self.legacy.get(&key).await? {
                if self.primary.get(&key).await?.is_none() {
                    self.primary.set(&key, &value).await?;
                }
                self.legacy.delete(&key).await?;
                count += 1;
            }
        }
        self.migrated.store(true, Ordering::Release);
        Ok(count)
    }
}

#[async_trait]
impl SecretStore for MigratingSecretStore {
    async fn get(&self, key: &str) -> VesselResult<Option<String>> {
        self.migrate_legacy_secrets().await?;
        if let Some(val) = self.primary.get(key).await? {
            return Ok(Some(val));
        }
        if let Some(legacy_val) = self.legacy.get(key).await? {
            self.primary.set(key, &legacy_val).await?;
            self.legacy.delete(key).await?;
            return Ok(Some(legacy_val));
        }
        Ok(None)
    }

    async fn set(&self, key: &str, value: &str) -> VesselResult<()> {
        self.migrate_legacy_secrets().await?;
        self.primary.set(key, value).await?;
        let _ = self.legacy.delete(key).await;
        Ok(())
    }

    async fn delete(&self, key: &str) -> VesselResult<()> {
        self.migrate_legacy_secrets().await?;
        self.primary.delete(key).await?;
        let _ = self.legacy.delete(key).await;
        Ok(())
    }

    async fn list_keys(&self) -> VesselResult<Vec<String>> {
        self.migrate_legacy_secrets().await?;
        let mut keys: BTreeSet<String> = self.primary.list_keys().await?.into_iter().collect();
        for key in self.legacy.list_keys().await? {
            keys.insert(key);
        }
        Ok(keys.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use async_trait::async_trait;
    use tokio::sync::Mutex;
    use vessel_domain::{SecretStore, VesselError, VesselResult};

    use super::{MigratingSecretStore, SqliteSecretStore};
    use crate::StorageActor;

    #[derive(Default)]
    struct MockVault {
        map: Mutex<BTreeMap<String, String>>,
    }

    #[async_trait]
    impl SecretStore for MockVault {
        async fn get(&self, key: &str) -> VesselResult<Option<String>> {
            Ok(self.map.lock().await.get(key).cloned())
        }

        async fn set(&self, key: &str, value: &str) -> VesselResult<()> {
            self.map
                .lock()
                .await
                .insert(key.to_owned(), value.to_owned());
            Ok(())
        }

        async fn delete(&self, key: &str) -> VesselResult<()> {
            self.map.lock().await.remove(key);
            Ok(())
        }

        async fn list_keys(&self) -> VesselResult<Vec<String>> {
            Ok(self.map.lock().await.keys().cloned().collect())
        }
    }

    #[tokio::test]
    async fn migrates_legacy_sqlite_secrets_into_primary_vault_and_clears_sqlite() {
        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let legacy = SqliteSecretStore::new(actor);
        legacy
            .set("debrid.real_debrid.token", "rd-secret-123")
            .await
            .expect("seed legacy token");
        legacy
            .set("metadata.tmdb_api_key", "tmdb-abc")
            .await
            .expect("seed legacy tmdb");

        let vault = Arc::new(MockVault::default());
        let store = MigratingSecretStore::new(vault.clone(), legacy.clone());

        let migrated = store.migrate_legacy_secrets().await.expect("migrate");
        assert_eq!(migrated, 2);
        assert_eq!(
            store.get("debrid.real_debrid.token").await.expect("get"),
            Some("rd-secret-123".into())
        );
        assert_eq!(
            vault.get("metadata.tmdb_api_key").await.expect("vault"),
            Some("tmdb-abc".into())
        );
        assert_eq!(legacy.list_keys().await.expect("legacy cleared").len(), 0);
        assert_eq!(
            store.list_keys().await.expect("list keys"),
            vec![
                "debrid.real_debrid.token".to_owned(),
                "metadata.tmdb_api_key".to_owned(),
            ]
        );
    }

    #[tokio::test]
    async fn performs_secret_crud_through_secret_store_trait() {
        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let store: Arc<dyn SecretStore> = Arc::new(SqliteSecretStore::new(actor));

        assert_eq!(
            store
                .get("anthology.api_key")
                .await
                .expect("missing secret"),
            None
        );

        store
            .set("anthology.api_key", "secret-v1")
            .await
            .expect("insert secret");
        assert_eq!(
            store.get("anthology.api_key").await.expect("read secret"),
            Some("secret-v1".into())
        );

        store
            .set("anthology.api_key", "secret-v2")
            .await
            .expect("upsert secret");
        assert_eq!(
            store.get("anthology.api_key").await.expect("read updated"),
            Some("secret-v2".into())
        );

        store
            .delete("anthology.api_key")
            .await
            .expect("delete secret");
        assert_eq!(
            store.get("anthology.api_key").await.expect("after delete"),
            None
        );

        assert_eq!(
            store.set("  ", "x").await,
            Err(VesselError::InvalidInput(
                "secret key cannot be empty".into()
            ))
        );
    }
}
