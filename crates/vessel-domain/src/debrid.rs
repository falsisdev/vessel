//! Debrid provider domain models, cloud status, resolution outcomes, and trait contracts.

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::VesselResult;
use crate::stream::TorrentMeta;

/// Built-in Debrid service identifiers supported natively by `vessel-debrid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebridProviderKind {
    RealDebrid,
    TorBox,
    AllDebrid,
    Premiumize,
    DebridLink,
}

impl DebridProviderKind {
    /// Returns all built-in Debrid providers in default fallback order.
    pub const fn all() -> [Self; 5] {
        [
            Self::RealDebrid,
            Self::TorBox,
            Self::AllDebrid,
            Self::Premiumize,
            Self::DebridLink,
        ]
    }

    /// Returns the canonical target ID used in priority lists (`builtin:<kind>`).
    pub const fn target_id(self) -> &'static str {
        match self {
            Self::RealDebrid => "builtin:real_debrid",
            Self::TorBox => "builtin:tor_box",
            Self::AllDebrid => "builtin:all_debrid",
            Self::Premiumize => "builtin:premiumize",
            Self::DebridLink => "builtin:debrid_link",
        }
    }

    /// Returns the human-readable provider label.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::RealDebrid => "Real-Debrid",
            Self::TorBox => "TorBox",
            Self::AllDebrid => "AllDebrid",
            Self::Premiumize => "Premiumize",
            Self::DebridLink => "Debrid-Link",
        }
    }

    /// Returns a short badge tag shown in stream lists.
    pub const fn short_tag(self) -> &'static str {
        match self {
            Self::RealDebrid => "RD",
            Self::TorBox => "TB",
            Self::AllDebrid => "AD",
            Self::Premiumize => "PM",
            Self::DebridLink => "DL",
        }
    }

    /// Parses a provider slug or `builtin:<slug>` target ID.
    pub fn parse_id(raw: &str) -> Option<Self> {
        let normalized = raw
            .trim()
            .strip_prefix("builtin:")
            .unwrap_or(raw.trim())
            .to_ascii_lowercase();
        match normalized.as_str() {
            "real_debrid" | "realdebrid" | "rd" => Some(Self::RealDebrid),
            "tor_box" | "torbox" | "tb" => Some(Self::TorBox),
            "all_debrid" | "alldebrid" | "ad" => Some(Self::AllDebrid),
            "premiumize" | "pm" => Some(Self::Premiumize),
            "debrid_link" | "debridlink" | "dl" => Some(Self::DebridLink),
            _ => None,
        }
    }
}

/// Instant cache availability state of a torrent on a Debrid provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebridCacheState {
    Cached,
    Uncached,
    Unknown,
}

/// Enriched cache availability hint attached to a torrent `StreamCandidate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebridCacheInfo {
    pub target: String,
    pub label: String,
    pub state: DebridCacheState,
}

/// Account verification & quota metadata for a built-in or community Debrid provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebridAccountStatus {
    pub target: String,
    pub label: String,
    pub is_builtin: bool,
    pub supports_bulk_cache_check: bool,
    pub enabled: bool,
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub is_premium: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_or_quota: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Lifecycle state of an active or completed torrent inside a Debrid cloud account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebridCloudStatus {
    Queued,
    Downloading,
    Uploading,
    Ready,
    Error,
}

/// Single torrent item inside a user's Debrid cloud transfer queue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebridCloudItem {
    pub id: String,
    pub target: String,
    pub provider_label: String,
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded_bytes: Option<u64>,
    pub status: DebridCloudStatus,
    pub progress_pct: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_bps: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeders: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u64>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_at: Option<String>,
}

/// Target file selection metadata passed to `DebridProvider::resolve_magnet`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode: Option<u32>,
}

impl From<&TorrentMeta> for TorrentTarget {
    fn from(meta: &TorrentMeta) -> Self {
        Self {
            info_hash: meta.info_hash.clone(),
            file_index: meta.file_index,
            filename: meta.filename.clone(),
            season: meta.season,
            episode: meta.episode,
        }
    }
}

/// Direct HTTP stream link returned after resolving a Debrid torrent or link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedHttpLink {
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Low-level resolution outcome returned by a single `DebridProvider`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum DebridResolveOutcome {
    Ready(ResolvedHttpLink),
    PendingCloud {
        #[serde(skip_serializing_if = "Option::is_none")]
        torrent_id: Option<String>,
        status: DebridCloudStatus,
        progress_pct: f32,
        #[serde(skip_serializing_if = "Option::is_none")]
        speed_bps: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        seeders: Option<u32>,
    },
}

/// Engine-level Debrid resolution result returned to UI callers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DebridResolution {
    #[serde(rename_all = "camelCase")]
    Ready {
        target: String,
        provider_label: String,
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        size_bytes: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    NeedsConfirmation {
        target: String,
        provider_label: String,
        info_hash: String,
        cloud_status: DebridCloudStatus,
        progress_pct: f32,
        #[serde(skip_serializing_if = "Option::is_none")]
        speed_bps: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        seeders: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        torrent_id: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    NoProviderConfigured { message: String },
}

/// Persisted Debrid prioritization and fallback preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebridPreferences {
    #[serde(default = "default_priority_order")]
    pub priority_order: Vec<String>,
    #[serde(default)]
    pub only_cached: bool,
    #[serde(default = "default_true")]
    pub auto_fallback: bool,
}

fn default_priority_order() -> Vec<String> {
    DebridProviderKind::all()
        .into_iter()
        .map(|k| k.target_id().to_owned())
        .collect()
}

fn default_true() -> bool {
    true
}

impl Default for DebridPreferences {
    fn default() -> Self {
        Self {
            priority_order: default_priority_order(),
            only_cached: false,
            auto_fallback: true,
        }
    }
}

/// Real-Debrid OAuth Device Code initiation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealDebridDeviceAuthStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_verification_url: Option<String>,
    pub interval_seconds: u64,
    pub expires_in_seconds: u64,
}

/// Real-Debrid OAuth Device Code poll response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealDebridDeviceAuthPoll {
    pub authorized: bool,
    pub pending: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request payload for resolving a torrent stream candidate via `VesselEngine`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveDebridRequest {
    pub url: String,
    #[serde(default)]
    pub torrent: Option<TorrentMeta>,
    #[serde(default)]
    pub preferred_target: Option<String>,
    #[serde(default)]
    pub allow_uncached_queue: bool,
    #[serde(default)]
    pub season: Option<u32>,
    #[serde(default)]
    pub episode: Option<u32>,
}

/// Unified Debrid provider contract implemented by built-in Rust providers and QuickJS plugins.
#[async_trait]
pub trait DebridProvider: Send + Sync {
    fn target_id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn is_builtin(&self) -> bool;
    fn supports_bulk_cache_check(&self) -> bool;

    async fn account_status(&self) -> VesselResult<DebridAccountStatus>;
    async fn check_cached(&self, hashes: &[String]) -> VesselResult<HashMap<String, bool>>;
    async fn resolve_magnet(
        &self,
        magnet_or_hash: &str,
        target: &TorrentTarget,
        allow_uncached_queue: bool,
    ) -> VesselResult<DebridResolveOutcome>;
    async fn list_cloud(&self) -> VesselResult<Vec<DebridCloudItem>>;
    async fn delete_cloud(&self, torrent_id: &str) -> VesselResult<()>;
    async fn unrestrict_link(&self, link: &str) -> VesselResult<ResolvedHttpLink>;
}

#[cfg(test)]
mod tests {
    use super::{DebridPreferences, DebridProviderKind, DebridResolution};

    #[test]
    fn parses_debrid_provider_ids_and_defaults() {
        assert_eq!(
            DebridProviderKind::parse_id("builtin:real_debrid"),
            Some(DebridProviderKind::RealDebrid)
        );
        assert_eq!(
            DebridProviderKind::parse_id("torbox"),
            Some(DebridProviderKind::TorBox)
        );
        assert_eq!(
            DebridProviderKind::parse_id("dl"),
            Some(DebridProviderKind::DebridLink)
        );

        let prefs = DebridPreferences::default();
        assert_eq!(prefs.priority_order.len(), 5);
        assert!(prefs.auto_fallback);
        assert!(!prefs.only_cached);

        let res = DebridResolution::NoProviderConfigured {
            message: "Connect a Debrid account".into(),
        };
        let raw = serde_json::to_string(&res).expect("serialize");
        assert!(raw.contains(r#""status":"no_provider_configured""#));
    }
}
