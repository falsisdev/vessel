//! Port traits defining core boundaries (storage, providers, player, events).

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::VesselResult;
use crate::event::EngineEvent;
use crate::ids::{ExternalIds, MediaRef, PluginId};
use crate::media::{Episode, MediaDetail, MediaSummary, MediaType, PlaybackProgress};
use crate::plugin::PluginManifest;
use crate::stream::{StreamResult, SubtitleFormat, SubtitleSource, SubtitleTrack};

/// Visual card layout hint for a home feed shelf.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeSectionLayout {
    #[default]
    Poster,
    Landscape,
}

/// Single curated shelf returned by `ContentProvider::home_feed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSection {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub layout: HomeSectionLayout,
    pub items: Vec<MediaSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Home feed response returned by a content provider.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeFeed {
    #[serde(default)]
    pub sections: Vec<HomeSection>,
}

/// Paginated search response returned by a content provider.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    #[serde(default)]
    pub items: Vec<MediaSummary>,
    #[serde(default)]
    pub has_more: bool,
}

/// Query passed to independent subtitle provider plugins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleQuery {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    pub media_type: MediaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode: Option<u32>,
    #[serde(default)]
    pub external_ids: ExternalIds,
    #[serde(default)]
    pub preferred_languages: Vec<String>,
}

/// Candidate returned by `SubtitleProvider::search_subtitles`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleCandidate {
    pub id: String,
    pub lang: String,
    pub label: String,
    pub format: SubtitleFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hearing_impaired: Option<bool>,
}

/// Resolved subtitle payload returned by `SubtitleProvider::resolve_subtitle`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSubtitle {
    pub format: SubtitleFormat,
    pub source: SubtitleSource,
}

/// User library watchlist status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryStatus {
    Watching,
    PlanToWatch,
    Completed,
    OnHold,
    Dropped,
}

/// Persisted library item linked to a self-contained `MediaRef`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub media: MediaRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_id: Option<String>,
    pub media_type: MediaType,
    #[serde(default)]
    pub is_anime: bool,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    pub status: LibraryStatus,
    pub added_at: String,
    pub updated_at: String,
}

/// Origin of a plugin domain allowlist entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainGrantSource {
    Manifest,
    UserPermanent,
}

/// Normalized playback request passed to a platform `PlayerBackend`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackRequest {
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub media_type: MediaType,
    pub start_position_ms: u64,
    #[serde(default)]
    pub subtitles: Vec<SubtitleTrack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Self-contained VOD or custom Live TV provider contract.
#[async_trait]
pub trait ContentProvider: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    async fn home_feed(&self, cursor: Option<&str>) -> VesselResult<HomeFeed>;
    async fn search(
        &self,
        query: &str,
        filters: &BTreeMap<String, String>,
        page: u32,
    ) -> VesselResult<SearchPage>;
    async fn details(&self, native_id: &str) -> VesselResult<MediaDetail>;
    async fn season_episodes(&self, native_id: &str, season_id: &str)
    -> VesselResult<Vec<Episode>>;
    async fn streams(&self, stream_id: &str) -> VesselResult<StreamResult>;
}

/// Independent subtitle provider contract.
#[async_trait]
pub trait SubtitleProvider: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    async fn search_subtitles(&self, query: &SubtitleQuery)
    -> VesselResult<Vec<SubtitleCandidate>>;
    async fn resolve_subtitle(&self, subtitle_id: &str) -> VesselResult<ResolvedSubtitle>;
}

/// Persistent storage port implemented by `vessel-storage`.
#[async_trait]
pub trait StoragePort: Send + Sync {
    async fn get_setting(&self, key: &str) -> VesselResult<Option<String>>;
    async fn set_setting(&self, key: &str, value_json: &str) -> VesselResult<()>;

    async fn get_plugin_kv(&self, plugin_id: &PluginId, key: &str) -> VesselResult<Option<String>>;
    async fn set_plugin_kv(&self, plugin_id: &PluginId, key: &str, value: &str)
    -> VesselResult<()>;
    async fn remove_plugin_kv(&self, plugin_id: &PluginId, key: &str) -> VesselResult<()>;

    async fn is_domain_allowed(&self, plugin_id: &PluginId, domain: &str) -> VesselResult<bool>;
    async fn grant_domain(
        &self,
        plugin_id: &PluginId,
        domain: &str,
        source: DomainGrantSource,
    ) -> VesselResult<()>;

    async fn get_progress(
        &self,
        media: &MediaRef,
        season: u32,
        episode: u32,
    ) -> VesselResult<Option<PlaybackProgress>>;
    async fn upsert_progress(&self, progress: &PlaybackProgress) -> VesselResult<()>;

    async fn list_library(&self, status: Option<LibraryStatus>) -> VesselResult<Vec<LibraryItem>>;
    async fn upsert_library_item(&self, item: &LibraryItem) -> VesselResult<()>;
    async fn remove_library_item(&self, media: &MediaRef) -> VesselResult<()>;
}

/// Sensitive credentials store (`SqliteSecretStore` in v0.1, `OsKeyringSecretStore` in v1.0).
#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn get(&self, key: &str) -> VesselResult<Option<String>>;
    async fn set(&self, key: &str, value: &str) -> VesselResult<()>;
    async fn delete(&self, key: &str) -> VesselResult<()>;
}

/// Platform-specific video playback backend (`libmpv`, `mpvkit`, `ExoPlayer`).
#[async_trait]
pub trait PlayerBackend: Send + Sync {
    async fn load(&self, request: PlaybackRequest) -> VesselResult<()>;
    async fn add_subtitle(&self, track: SubtitleTrack) -> VesselResult<()>;
    async fn set_subtitle_delay(&self, delay_ms: i64) -> VesselResult<()>;
    async fn play(&self) -> VesselResult<()>;
    async fn pause(&self) -> VesselResult<()>;
    async fn seek(&self, position_ms: u64) -> VesselResult<()>;
    async fn stop(&self) -> VesselResult<()>;
}

/// Non-blocking sink streaming core events to any UI adapter.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: EngineEvent);
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::EventSink;
    use crate::{EngineEvent, PluginId};

    struct RecordingSink {
        events: Mutex<Vec<EngineEvent>>,
    }

    impl EventSink for RecordingSink {
        fn emit(&self, event: EngineEvent) {
            self.events.lock().expect("lock events").push(event);
        }
    }

    #[test]
    fn dispatches_events_through_trait_object_sink() {
        let recorder = Arc::new(RecordingSink {
            events: Mutex::new(Vec::new()),
        });
        let sink: Arc<dyn EventSink> = recorder.clone();

        sink.emit(EngineEvent::DomainPermissionRequested {
            request_id: "req-1".into(),
            plugin_id: PluginId::new("anthology").expect("valid plugin id"),
            domain: "cdn.anthology.example".into(),
        });

        let recorded = recorder.events.lock().expect("lock events");
        assert_eq!(recorded.len(), 1);
        assert!(matches!(
            &recorded[0],
            EngineEvent::DomainPermissionRequested { domain, .. } if domain == "cdn.anthology.example"
        ));
    }
}
