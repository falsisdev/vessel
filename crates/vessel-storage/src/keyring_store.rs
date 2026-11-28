//! OS native keyring (`Keychain` / `Credential Manager` / `Secret Service`) `SecretStore`
//! implementation for desktop platforms.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use vessel_domain::{SecretStore, VesselError, VesselResult};

use crate::{MigratingSecretStore, SqliteSecretStore, SqliteStorage, StorageActor};

/// Default OS keyring service identifier for Vessel desktop builds.
pub const DEFAULT_KEYRING_SERVICE: &str = "dev.falsis.vessel";

/// Non-sensitive SQLite `settings` key tracking which secret keys exist in the OS keyring.
const KEYRING_INDEX_SETTING: &str = "vessel.keyring_key_index";

fn validate_secret_key(key: &str) -> VesselResult<String> {
    let trimmed = key.trim().to_owned();
    if trimmed.is_empty() {
        return Err(VesselError::InvalidInput(
            "secret key cannot be empty".into(),
        ));
    }
    Ok(trimmed)
}

fn map_keyring_err(action: &str, err: keyring::Error) -> VesselError {
    VesselError::Storage(format!("os keyring {action} failed: {err}"))
}

/// Persists sensitive credentials in the operating system's native credential vault
/// (`macOS Keychain`, `Windows Credential Manager`, or `Linux Secret Service` via pure-Rust D-Bus).
#[derive(Debug, Clone)]
pub struct OsKeyringSecretStore {
    service_name: String,
    storage: SqliteStorage,
    index_lock: Arc<Mutex<()>>,
}

impl OsKeyringSecretStore {
    /// Creates an `OsKeyringSecretStore` bound to `DEFAULT_KEYRING_SERVICE`.
    pub fn new_default(actor: StorageActor) -> Self {
        Self::new(DEFAULT_KEYRING_SERVICE, actor)
    }

    /// Creates an `OsKeyringSecretStore` with a custom keyring service identifier.
    pub fn new(service_name: impl Into<String>, actor: StorageActor) -> Self {
        Self {
            service_name: service_name.into(),
            storage: SqliteStorage::new(actor),
            index_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Probes whether the host OS keyring daemon/vault is accessible in the current environment.
    pub async fn probe_available(&self) -> VesselResult<()> {
        if std::env::var_os("VESSEL_DISABLE_KEYRING").is_some() {
            return Err(VesselError::Storage(
                "os keyring disabled via VESSEL_DISABLE_KEYRING".into(),
            ));
        }
        let service = self.service_name.clone();
        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service, "__vessel_keyring_probe__")
                .map_err(|err| map_keyring_err("entry init", err))?;
            match entry.get_password() {
                Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(err) => Err(map_keyring_err("probe", err)),
            }
        })
        .await
        .map_err(|err| VesselError::Storage(format!("os keyring probe join error: {err}")))?
    }

    async fn update_key_index(&self, key: &str, present: bool) -> VesselResult<()> {
        let _guard = self.index_lock.lock().await;
        let mut keys: BTreeSet<String> = self
            .storage
            .get_json_setting::<Vec<String>>(KEYRING_INDEX_SETTING)
            .await?
            .unwrap_or_default()
            .into_iter()
            .collect();

        let changed = if present {
            keys.insert(key.to_owned())
        } else {
            keys.remove(key)
        };

        if changed {
            let ordered: Vec<String> = keys.into_iter().collect();
            self.storage
                .set_json_setting(KEYRING_INDEX_SETTING, &ordered)
                .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl SecretStore for OsKeyringSecretStore {
    async fn get(&self, key: &str) -> VesselResult<Option<String>> {
        let key = validate_secret_key(key)?;
        let service = self.service_name.clone();
        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service, &key)
                .map_err(|err| map_keyring_err("entry init", err))?;
            match entry.get_password() {
                Ok(secret) => Ok(Some(secret)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(err) => Err(map_keyring_err("get", err)),
            }
        })
        .await
        .map_err(|err| VesselError::Storage(format!("os keyring get join error: {err}")))?
    }

    async fn set(&self, key: &str, value: &str) -> VesselResult<()> {
        let key = validate_secret_key(key)?;
        let service = self.service_name.clone();
        let value = value.to_owned();
        let key_for_task = key.clone();

        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service, &key_for_task)
                .map_err(|err| map_keyring_err("entry init", err))?;
            entry
                .set_password(&value)
                .map_err(|err| map_keyring_err("set", err))
        })
        .await
        .map_err(|err| VesselError::Storage(format!("os keyring set join error: {err}")))??;

        self.update_key_index(&key, true).await
    }

    async fn delete(&self, key: &str) -> VesselResult<()> {
        let key = validate_secret_key(key)?;
        let service = self.service_name.clone();
        let key_for_task = key.clone();

        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service, &key_for_task)
                .map_err(|err| map_keyring_err("entry init", err))?;
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(err) => Err(map_keyring_err("delete", err)),
            }
        })
        .await
        .map_err(|err| VesselError::Storage(format!("os keyring delete join error: {err}")))??;

        self.update_key_index(&key, false).await
    }

    async fn list_keys(&self) -> VesselResult<Vec<String>> {
        let keys = self
            .storage
            .get_json_setting::<Vec<String>>(KEYRING_INDEX_SETTING)
            .await?
            .unwrap_or_default();
        Ok(keys)
    }
}

/// Opens the best available desktop `SecretStore` (`OsKeyringSecretStore` + one-time SQLite migration),
/// falling back gracefully to `SqliteSecretStore` only when the host OS has no accessible keyring daemon.
pub async fn open_desktop_secret_store(actor: StorageActor) -> Arc<dyn SecretStore> {
    let legacy = SqliteSecretStore::new(actor.clone());
    let keyring_store = OsKeyringSecretStore::new_default(actor);
    if keyring_store.probe_available().await.is_ok() {
        let migrating = Arc::new(MigratingSecretStore::new(
            Arc::new(keyring_store),
            legacy.clone(),
        ));
        if migrating.migrate_legacy_secrets().await.is_ok() {
            return migrating;
        }
    }
    Arc::new(legacy)
}
