//! Google Drive `appDataFolder` incremental sync client (LWW + SQLite tombstones).

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use vessel_auth::OAuthManager;
use vessel_domain::{
    CloudSyncStatus, OAuthProviderKind, VesselError, VesselResult, http_client_builder,
};
use vessel_storage::{CloudSyncSnapshot, SqliteStorage, SyncMergeStats};

const SNAPSHOT_FILE_NAME: &str = "vessel-sync-snapshot-v2.json";
const STATE_EMAIL: &str = "google_drive_email";
const STATE_LAST_SYNCED: &str = "google_drive_last_synced_at";
const STATE_LAST_ERROR: &str = "google_drive_last_error";
const STATE_AUTO_SYNC: &str = "google_drive_auto_sync";

/// Outcome returned by a full two-way Google Drive `appDataFolder` sync cycle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncResult {
    pub uploaded_file_id: String,
    pub merge: SyncMergeStatsReport,
    pub status: CloudSyncStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMergeStatsReport {
    pub library_applied: usize,
    pub progress_applied: usize,
    pub overrides_applied: usize,
}

impl From<SyncMergeStats> for SyncMergeStatsReport {
    fn from(s: SyncMergeStats) -> Self {
        Self {
            library_applied: s.library_applied,
            progress_applied: s.progress_applied,
            overrides_applied: s.overrides_applied,
        }
    }
}

/// Synchronizes `CloudSyncSnapshot` JSON inside the user's private Google Drive `appDataFolder`.
#[derive(Clone)]
pub struct GoogleDriveSyncClient {
    oauth: Arc<OAuthManager>,
    storage: Arc<SqliteStorage>,
    http: reqwest::Client,
    drive_api_base: String,
    upload_api_base: String,
    userinfo_url: String,
}

impl std::fmt::Debug for GoogleDriveSyncClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoogleDriveSyncClient")
            .field("drive_api_base", &self.drive_api_base)
            .finish_non_exhaustive()
    }
}

impl GoogleDriveSyncClient {
    pub fn new(oauth: Arc<OAuthManager>, storage: Arc<SqliteStorage>) -> VesselResult<Self> {
        Self::with_endpoints(
            oauth,
            storage,
            "https://www.googleapis.com/drive/v3",
            "https://www.googleapis.com/upload/drive/v3",
            "https://www.googleapis.com/oauth2/v2/userinfo",
        )
    }

    pub fn with_endpoints(
        oauth: Arc<OAuthManager>,
        storage: Arc<SqliteStorage>,
        drive_api_base: impl Into<String>,
        upload_api_base: impl Into<String>,
        userinfo_url: impl Into<String>,
    ) -> VesselResult<Self> {
        let http = http_client_builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|err| VesselError::Network(format!("build drive sync client: {err}")))?;
        Ok(Self {
            oauth,
            storage,
            http,
            drive_api_base: drive_api_base.into().trim_end_matches('/').to_owned(),
            upload_api_base: upload_api_base.into().trim_end_matches('/').to_owned(),
            userinfo_url: userinfo_url.into(),
        })
    }

    /// Reads current Google Drive cloud sync status without hitting the network.
    pub async fn status(&self) -> VesselResult<CloudSyncStatus> {
        let has_client_config = self
            .oauth
            .get_client_config(OAuthProviderKind::GoogleDrive)
            .await?
            .is_some();
        let connected = self
            .oauth
            .get_token_bundle(OAuthProviderKind::GoogleDrive)
            .await?
            .is_some();
        let device_id = self.storage.get_or_create_sync_device_id().await?;
        let account_email = self.storage.get_sync_state(STATE_EMAIL).await?;
        let last_synced_at = self.storage.get_sync_state(STATE_LAST_SYNCED).await?;
        let last_error = self
            .storage
            .get_sync_state(STATE_LAST_ERROR)
            .await?
            .filter(|s| !s.is_empty());
        let auto_sync = self
            .storage
            .get_sync_state(STATE_AUTO_SYNC)
            .await?
            .is_none_or(|v| v != "false");
        let active_tombstones = self.storage.count_active_tombstones().await?;

        Ok(CloudSyncStatus {
            connected,
            has_client_config,
            account_email,
            device_id,
            auto_sync,
            last_synced_at,
            last_error,
            active_tombstones,
        })
    }

    pub async fn set_auto_sync(&self, enabled: bool) -> VesselResult<()> {
        self.storage
            .set_sync_state(STATE_AUTO_SYNC, if enabled { "true" } else { "false" })
            .await
    }

    pub async fn disconnect(&self) -> VesselResult<()> {
        self.oauth.disconnect(OAuthProviderKind::GoogleDrive).await?;
        self.storage.set_sync_state(STATE_EMAIL, "").await?;
        self.storage.set_sync_state(STATE_LAST_ERROR, "").await?;
        Ok(())
    }

    /// Refreshes cached Google account email if missing.
    pub async fn refresh_account_email(&self) -> VesselResult<Option<String>> {
        let token = self
            .oauth
            .get_valid_access_token(OAuthProviderKind::GoogleDrive)
            .await?;
        let resp = self
            .http
            .get(&self.userinfo_url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("google userinfo request: {err}")))?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        #[derive(Deserialize)]
        struct UserInfo {
            email: Option<String>,
        }
        let info: UserInfo = resp
            .json()
            .await
            .map_err(|err| VesselError::Network(format!("decode google userinfo: {err}")))?;
        if let Some(ref email) = info.email {
            let _ = self.storage.set_sync_state(STATE_EMAIL, email).await;
        }
        Ok(info.email)
    }

    /// Runs a two-way LWW + tombstone sync against `appDataFolder/vessel-sync-snapshot-v2.json`.
    pub async fn sync_now(&self) -> VesselResult<CloudSyncResult> {
        let outcome = self.sync_now_inner().await;
        match &outcome {
            Ok(_) => {
                let _ = self.storage.set_sync_state(STATE_LAST_ERROR, "").await;
                let _ = self
                    .storage
                    .set_sync_state(STATE_LAST_SYNCED, &utc_iso_now())
                    .await;
            }
            Err(err) => {
                let _ = self
                    .storage
                    .set_sync_state(STATE_LAST_ERROR, &err.to_string())
                    .await;
            }
        }
        outcome
    }

    async fn sync_now_inner(&self) -> VesselResult<CloudSyncResult> {
        let token = self
            .oauth
            .get_valid_access_token(OAuthProviderKind::GoogleDrive)
            .await?;

        if self
            .storage
            .get_sync_state(STATE_EMAIL)
            .await?
            .as_deref()
            .is_none_or(str::is_empty)
        {
            let _ = self.refresh_account_email().await;
        }

        let existing_id = self.find_snapshot_file_id(&token).await?;
        let merge_stats = if let Some(ref file_id) = existing_id {
            let remote = self.download_snapshot(&token, file_id).await?;
            self.storage.merge_sync_snapshot(remote).await?
        } else {
            SyncMergeStats::default()
        };

        let merged_snapshot = self.storage.export_sync_snapshot().await?;
        let uploaded_file_id = if let Some(file_id) = existing_id {
            self.patch_snapshot(&token, &file_id, &merged_snapshot)
                .await?;
            file_id
        } else {
            self.create_snapshot(&token, &merged_snapshot).await?
        };

        let _ = self
            .storage
            .set_sync_state(STATE_LAST_SYNCED, &utc_iso_now())
            .await;
        let _ = self.storage.set_sync_state(STATE_LAST_ERROR, "").await;
        let status = self.status().await?;

        Ok(CloudSyncResult {
            uploaded_file_id,
            merge: merge_stats.into(),
            status,
        })
    }

    async fn find_snapshot_file_id(&self, token: &str) -> VesselResult<Option<String>> {
        let resp = self
            .http
            .get(format!("{}/files", self.drive_api_base))
            .bearer_auth(token)
            .query(&[
                ("spaces", "appDataFolder"),
                ("q", "name='vessel-sync-snapshot-v2.json' and trashed=false"),
                ("fields", "files(id,name,modifiedTime)"),
                ("pageSize", "1"),
            ])
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("drive list appDataFolder: {err}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|err| VesselError::Network(format!("read drive list response: {err}")))?;
        if !status.is_success() {
            return Err(VesselError::Network(format!(
                "drive list HTTP {status}: {}",
                text.trim()
            )));
        }

        #[derive(Deserialize)]
        struct DriveFile {
            id: String,
        }
        #[derive(Deserialize)]
        struct ListResp {
            #[serde(default)]
            files: Vec<DriveFile>,
        }
        let parsed: ListResp = serde_json::from_str(&text)
            .map_err(|err| VesselError::Network(format!("decode drive list JSON: {err}")))?;
        Ok(parsed.files.into_iter().next().map(|f| f.id))
    }

    async fn download_snapshot(
        &self,
        token: &str,
        file_id: &str,
    ) -> VesselResult<CloudSyncSnapshot> {
        let resp = self
            .http
            .get(format!("{}/files/{file_id}", self.drive_api_base))
            .bearer_auth(token)
            .query(&[("alt", "media")])
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("drive download snapshot: {err}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|err| VesselError::Network(format!("read drive snapshot: {err}")))?;
        if !status.is_success() {
            return Err(VesselError::Network(format!(
                "drive download HTTP {status}: {}",
                text.trim()
            )));
        }
        serde_json::from_str(&text)
            .map_err(|err| VesselError::Network(format!("decode drive snapshot JSON: {err}")))
    }

    async fn create_snapshot(
        &self,
        token: &str,
        snapshot: &CloudSyncSnapshot,
    ) -> VesselResult<String> {
        let metadata_json = serde_json::json!({
            "name": SNAPSHOT_FILE_NAME,
            "parents": ["appDataFolder"],
            "mimeType": "application/json"
        })
        .to_string();
        let body_json = serde_json::to_string(snapshot)
            .map_err(|err| VesselError::Storage(format!("encode sync snapshot: {err}")))?;

        let boundary = "vessel_sync_boundary_7f9a2b";
        let multipart_body = format!(
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{metadata_json}\r\n--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{body_json}\r\n--{boundary}--\r\n"
        );

        let resp = self
            .http
            .post(format!("{}/files", self.upload_api_base))
            .bearer_auth(token)
            .query(&[("uploadType", "multipart"), ("fields", "id")])
            .header(
                "Content-Type",
                format!("multipart/related; boundary={boundary}"),
            )
            .body(multipart_body)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("drive create snapshot: {err}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|err| VesselError::Network(format!("read drive create response: {err}")))?;
        if !status.is_success() {
            return Err(VesselError::Network(format!(
                "drive create HTTP {status}: {}",
                text.trim()
            )));
        }

        #[derive(Deserialize)]
        struct Created {
            id: String,
        }
        let parsed: Created = serde_json::from_str(&text)
            .map_err(|err| VesselError::Network(format!("decode drive create JSON: {err}")))?;
        Ok(parsed.id)
    }

    async fn patch_snapshot(
        &self,
        token: &str,
        file_id: &str,
        snapshot: &CloudSyncSnapshot,
    ) -> VesselResult<()> {
        let body_json = serde_json::to_string(snapshot)
            .map_err(|err| VesselError::Storage(format!("encode sync snapshot: {err}")))?;
        let resp = self
            .http
            .patch(format!("{}/files/{file_id}", self.upload_api_base))
            .bearer_auth(token)
            .query(&[("uploadType", "media")])
            .header("Content-Type", "application/json; charset=UTF-8")
            .body(body_json)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("drive patch snapshot: {err}")))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(VesselError::Network(format!(
                "drive patch HTTP {status}: {}",
                text.trim()
            )));
        }
        Ok(())
    }
}

fn utc_iso_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // Convert epoch seconds to YYYY-MM-DDTHH:MM:SSZ using civil-from-days algorithm.
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hour = rem / 3_600;
    let min = (rem % 3_600) / 60;
    let sec = rem % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + i64::from(m <= 2);
    format!("{year:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use axum::Router;
    use axum::body::Bytes;
    use axum::extract::{Path, Query};
    use axum::http::StatusCode;
    use axum::routing::{get, patch, post};
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use vessel_auth::OAuthManager;
    use vessel_domain::{
        LibraryItem, LibraryStatus, MediaRef, MediaType, OAuthClientConfig, OAuthProviderKind,
        OAuthTokenBundle, PluginId, SecretStore, StoragePort, VesselResult,
    };
    use vessel_storage::{SqliteStorage, StorageActor};

    use super::GoogleDriveSyncClient;

    #[derive(Default)]
    struct MemSecrets {
        map: Mutex<BTreeMap<String, String>>,
    }

    #[async_trait::async_trait]
    impl SecretStore for MemSecrets {
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
    }

    #[tokio::test]
    async fn performs_drive_appdata_create_and_two_way_merge() {
        let remote_blob = Arc::new(Mutex::new(Option::<String>::None));
        let blob_list = Arc::clone(&remote_blob);
        let blob_get = Arc::clone(&remote_blob);
        let blob_post = Arc::clone(&remote_blob);
        let blob_patch = Arc::clone(&remote_blob);

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let app = Router::new()
            .route(
                "/oauth2/v2/userinfo",
                get(|| async { axum::Json(serde_json::json!({ "email": "pilot@vessel.local" })) }),
            )
            .route(
                "/drive/v3/files",
                get(move || {
                    let store = Arc::clone(&blob_list);
                    async move {
                        let guard = store.lock().await;
                        if guard.is_some() {
                            axum::Json(serde_json::json!({ "files": [{ "id": "file-1" }] }))
                        } else {
                            axum::Json(serde_json::json!({ "files": [] }))
                        }
                    }
                }),
            )
            .route(
                "/drive/v3/files/{id}",
                get(move |Path(_id): Path<String>, Query(_q): Query<BTreeMap<String, String>>| {
                    let store = Arc::clone(&blob_get);
                    async move {
                        match store.lock().await.clone() {
                            Some(json) => (StatusCode::OK, json),
                            None => (StatusCode::NOT_FOUND, "missing".into()),
                        }
                    }
                }),
            )
            .route(
                "/upload/drive/v3/files",
                post(move |body: Bytes| {
                    let store = Arc::clone(&blob_post);
                    async move {
                        let raw = String::from_utf8_lossy(&body);
                        // Extract second JSON part from multipart/related payload.
                        let snap_json = raw
                            .split("Content-Type: application/json; charset=UTF-8\r\n\r\n")
                            .nth(2)
                            .and_then(|rest| rest.split("\r\n--").next())
                            .expect("snapshot part in multipart")
                            .to_owned();
                        *store.lock().await = Some(snap_json);
                        axum::Json(serde_json::json!({ "id": "file-1" }))
                    }
                }),
            )
            .route(
                "/upload/drive/v3/files/{id}",
                patch(move |Path(_id): Path<String>, body: Bytes| {
                    let store = Arc::clone(&blob_patch);
                    async move {
                        *store.lock().await = Some(String::from_utf8_lossy(&body).into_owned());
                        StatusCode::OK
                    }
                }),
            );

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let base = format!("http://{addr}");
        let make_client = |storage: Arc<SqliteStorage>| {
            let secrets: Arc<dyn SecretStore> = Arc::new(MemSecrets::default());
            let oauth = Arc::new(OAuthManager::new(secrets).expect("oauth"));
            (oauth, storage)
        };

        // Device 1 adds an item and syncs -> uploads snapshot via multipart POST.
        let actor1 = StorageActor::open_in_memory().await.expect("actor1");
        let storage1 = Arc::new(SqliteStorage::new(actor1));
        let (oauth1, storage1) = make_client(storage1);
        oauth1
            .save_client_config(
                OAuthProviderKind::GoogleDrive,
                OAuthClientConfig {
                    client_id: "g-cid".into(),
                    client_secret: None,
                },
            )
            .await
            .expect("cfg1");
        oauth1
            .save_token_bundle(
                OAuthProviderKind::GoogleDrive,
                &OAuthTokenBundle {
                    access_token: "tok-1".into(),
                    refresh_token: None,
                    token_type: "Bearer".into(),
                    scope: None,
                    expires_at_epoch_secs: Some(9_999_999_999),
                },
            )
            .await
            .expect("tok1");

        let plugin = PluginId::new("anthology").expect("plugin");
        storage1
            .upsert_library(&LibraryItem {
                media: MediaRef::new(plugin.clone(), "/movie/inception").expect("ref"),
                canonical_id: Some("tmdb:movie:27205".into()),
                media_type: MediaType::Movie,
                is_anime: false,
                title: "Inception".into(),
                poster_url: None,
                year: Some(2010),
                status: LibraryStatus::Completed,
                added_at: "2026-11-28T10:00:00Z".into(),
                updated_at: "2026-11-28T10:00:00Z".into(),
            })
            .await
            .expect("upsert1");

        let client1 = GoogleDriveSyncClient::with_endpoints(
            oauth1,
            Arc::clone(&storage1),
            format!("{base}/drive/v3"),
            format!("{base}/upload/drive/v3"),
            format!("{base}/oauth2/v2/userinfo"),
        )
        .expect("client1");
        let res1 = client1.sync_now().await.expect("sync1");
        assert_eq!(res1.uploaded_file_id, "file-1");
        assert_eq!(res1.status.account_email.as_deref(), Some("pilot@vessel.local"));

        // Device 2 starts empty and syncs -> pulls Inception from Drive!
        let actor2 = StorageActor::open_in_memory().await.expect("actor2");
        let storage2 = Arc::new(SqliteStorage::new(actor2));
        let (oauth2, storage2) = make_client(storage2);
        oauth2
            .save_client_config(
                OAuthProviderKind::GoogleDrive,
                OAuthClientConfig {
                    client_id: "g-cid".into(),
                    client_secret: None,
                },
            )
            .await
            .expect("cfg2");
        oauth2
            .save_token_bundle(
                OAuthProviderKind::GoogleDrive,
                &OAuthTokenBundle {
                    access_token: "tok-2".into(),
                    refresh_token: None,
                    token_type: "Bearer".into(),
                    scope: None,
                    expires_at_epoch_secs: Some(9_999_999_999),
                },
            )
            .await
            .expect("tok2");

        let client2 = GoogleDriveSyncClient::with_endpoints(
            oauth2,
            Arc::clone(&storage2),
            format!("{base}/drive/v3"),
            format!("{base}/upload/drive/v3"),
            format!("{base}/oauth2/v2/userinfo"),
        )
        .expect("client2");
        let res2 = client2.sync_now().await.expect("sync2");
        assert_eq!(res2.merge.library_applied, 1);
        assert_eq!(storage2.list_library(None).await.expect("lib2").len(), 1);
    }
}
