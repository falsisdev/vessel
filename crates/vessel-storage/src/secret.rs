//! SQLite-backed `SecretStore` implementation (`v0.1` unsigned dev mode).

use async_trait::async_trait;
use rusqlite::OptionalExtension;
use vessel_domain::{SecretStore, VesselError, VesselResult};

use crate::{StorageActor, map_sql_err};

/// Persists sensitive values in the SQLite `secrets` table via `StorageActor`.
///
/// Intended for unsigned `v0.1` development builds where OS Keychain prompts would
/// interrupt local iteration; signed `v1.0` releases swap in `OsKeyringSecretStore`.
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
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use vessel_domain::{SecretStore, VesselError};

    use super::SqliteSecretStore;
    use crate::StorageActor;

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
