//! Shared OAuth 2.0, tracker sync, and cloud sync domain models.

use serde::{Deserialize, Serialize};

use crate::ids::ExternalIds;
use crate::media::MediaType;
use crate::ports::LibraryStatus;

/// Providers authenticated through Vessel's unified OAuth 2.0 subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProviderKind {
    GoogleDrive,
    Trakt,
    Simkl,
    Anilist,
    MyAnimeList,
    Kitsu,
}

impl OAuthProviderKind {
    /// Canonical storage/secret key prefix (`oauth.<id>.tokens`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GoogleDrive => "google_drive",
            Self::Trakt => "trakt",
            Self::Simkl => "simkl",
            Self::Anilist => "anilist",
            Self::MyAnimeList => "myanimelist",
            Self::Kitsu => "kitsu",
        }
    }

    /// SecretStore key holding the serialized [`OAuthTokenBundle`].
    pub fn token_secret_key(self) -> String {
        format!("oauth.{}.tokens", self.as_str())
    }

    /// SecretStore key holding optional user-supplied OAuth client credentials.
    pub fn client_config_secret_key(self) -> String {
        format!("oauth.{}.client_config", self.as_str())
    }
}

/// Optional user/app OAuth client configuration stored in `SecretStore`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthClientConfig {
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
}

/// Persisted OAuth 2.0 access/refresh token bundle stored exclusively in `SecretStore`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthTokenBundle {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default = "default_token_type")]
    pub token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at_epoch_secs: Option<i64>,
}

fn default_token_type() -> String {
    "Bearer".into()
}

impl OAuthTokenBundle {
    /// Returns `true` if `expires_at_epoch_secs` is within `skew_secs` of `now_epoch_secs`.
    pub fn is_expiring_soon(&self, now_epoch_secs: i64, skew_secs: i64) -> bool {
        self.expires_at_epoch_secs
            .is_some_and(|exp| now_epoch_secs.saturating_add(skew_secs) >= exp)
    }
}

/// Supported OAuth interactive flow modes across Desktop, Android, and Android TV.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthFlowKind {
    LoopbackPkce,
    AppLinkPkce,
    DeviceCode,
}

/// Device Code / QR verification prompt shown on TV or headless flows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthDevicePrompt {
    pub provider: OAuthProviderKind,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_uri_complete: Option<String>,
    pub expires_in_secs: u64,
    pub interval_secs: u64,
}

/// Result returned when starting an interactive OAuth authorization flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OAuthStartResult {
    #[serde(rename_all = "camelCase")]
    BrowserUrl {
        provider: OAuthProviderKind,
        auth_url: String,
        state: String,
        redirect_uri: String,
    },
    DevicePrompt(OAuthDevicePrompt),
}

/// Supported media watch-progress & library tracking services.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackerProviderKind {
    Trakt,
    Simkl,
    Anilist,
    MyAnimeList,
    Kitsu,
}

impl TrackerProviderKind {
    /// Converts to the underlying [`OAuthProviderKind`].
    pub fn oauth_provider(self) -> OAuthProviderKind {
        match self {
            Self::Trakt => OAuthProviderKind::Trakt,
            Self::Simkl => OAuthProviderKind::Simkl,
            Self::Anilist => OAuthProviderKind::Anilist,
            Self::MyAnimeList => OAuthProviderKind::MyAnimeList,
            Self::Kitsu => OAuthProviderKind::Kitsu,
        }
    }

    /// Canonical lowercase identifier.
    pub fn as_str(self) -> &'static str {
        self.oauth_provider().as_str()
    }

    /// Human-readable service title.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Trakt => "Trakt",
            Self::Simkl => "Simkl",
            Self::Anilist => "AniList",
            Self::MyAnimeList => "MyAnimeList",
            Self::Kitsu => "Kitsu",
        }
    }
}

/// Connection & synchronization state for a single tracker service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackerAccountStatus {
    pub provider: TrackerProviderKind,
    pub display_name: String,
    pub implemented: bool,
    pub has_client_config: bool,
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub auto_scrobble: bool,
    pub sync_library: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
    #[serde(default)]
    pub pending_queue_count: usize,
}

/// Normalized watchlist status mapped across all trackers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackerSyncStatus {
    Watching,
    Completed,
    PlanToWatch,
    OnHold,
    Dropped,
}

impl From<LibraryStatus> for TrackerSyncStatus {
    fn from(status: LibraryStatus) -> Self {
        match status {
            LibraryStatus::Watching => Self::Watching,
            LibraryStatus::PlanToWatch => Self::PlanToWatch,
            LibraryStatus::Completed => Self::Completed,
            LibraryStatus::OnHold => Self::OnHold,
            LibraryStatus::Dropped => Self::Dropped,
        }
    }
}

/// Outbound scrobble or library mutation queued/dispatched to connected trackers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum TrackerSyncAction {
    #[serde(rename_all = "camelCase")]
    Scrobble {
        external_ids: ExternalIds,
        title: String,
        media_type: MediaType,
        is_anime: bool,
        season: u32,
        episode: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        absolute_episode: Option<u32>,
        progress_pct: f32,
        completed: bool,
    },
    #[serde(rename_all = "camelCase")]
    SetLibraryStatus {
        external_ids: ExternalIds,
        title: String,
        media_type: MediaType,
        is_anime: bool,
        status: TrackerSyncStatus,
    },
}

/// Google Drive cross-device synchronization summary exposed to Settings UI.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncStatus {
    pub connected: bool,
    pub has_client_config: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_email: Option<String>,
    pub device_id: String,
    pub auto_sync: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default)]
    pub active_tombstones: usize,
}

#[cfg(test)]
mod tests {
    use super::{OAuthProviderKind, OAuthTokenBundle, TrackerProviderKind};

    #[test]
    fn token_expiry_skew_check_respects_window() {
        let bundle = OAuthTokenBundle {
            access_token: "tok".into(),
            refresh_token: Some("ref".into()),
            token_type: "Bearer".into(),
            scope: None,
            expires_at_epoch_secs: Some(1_000),
        };
        assert!(!bundle.is_expiring_soon(800, 120));
        assert!(bundle.is_expiring_soon(880, 120));
        assert!(bundle.is_expiring_soon(1_050, 120));
    }

    #[test]
    fn provider_keys_are_deterministic() {
        assert_eq!(
            OAuthProviderKind::GoogleDrive.token_secret_key(),
            "oauth.google_drive.tokens"
        );
        assert_eq!(
            TrackerProviderKind::Anilist.oauth_provider(),
            OAuthProviderKind::Anilist
        );
    }
}
