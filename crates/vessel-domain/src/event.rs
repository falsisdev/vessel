//! Engine events emitted to UI shells.

use serde::{Deserialize, Serialize};

use crate::ids::PluginId;
use crate::media::{MediaDetail, MediaSummary};

/// Log severity forwarded from sandboxed plugin consoles to UI dev tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Download lifecycle state exposed to the UI queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
}

/// Live snapshot of the active player backend state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStateSnapshot {
    pub is_playing: bool,
    pub is_paused: bool,
    pub is_buffering: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_stream_url: Option<String>,
    pub subtitle_delay_ms: i64,
}

/// Live progress snapshot of an offline download task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressSnapshot {
    pub download_id: String,
    pub status: DownloadStatus,
    pub bytes_downloaded: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    pub segments_done: u32,
    pub segments_total: u32,
}

/// Eight canonical events streamed from `VesselEngine` to any UI shell via `EventSink`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    #[serde(rename_all = "camelCase")]
    SearchPartialResult {
        search_id: String,
        plugin_id: PluginId,
        items: Vec<MediaSummary>,
    },
    #[serde(rename_all = "camelCase")]
    SearchCompleted {
        search_id: String,
    },
    #[serde(rename_all = "camelCase")]
    MetadataEnriched {
        plugin_id: PluginId,
        native_id: String,
        detail: Box<MediaDetail>,
    },
    PlayerStateChanged(PlayerStateSnapshot),
    #[serde(rename_all = "camelCase")]
    DomainPermissionRequested {
        request_id: String,
        plugin_id: PluginId,
        domain: String,
    },
    DownloadProgress(DownloadProgressSnapshot),
    #[serde(rename_all = "camelCase")]
    PluginUpdateAvailable {
        plugin_id: PluginId,
        current_version: String,
        latest_version: String,
    },
    #[serde(rename_all = "camelCase")]
    PluginLog {
        plugin_id: PluginId,
        level: LogLevel,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{EngineEvent, LogLevel};
    use crate::{MediaDetail, MediaType, PluginId};

    #[test]
    fn serializes_tagged_engine_events_cleanly() {
        let plugin_id = PluginId::new("anthology").expect("valid plugin id");
        let log_event = EngineEvent::PluginLog {
            plugin_id: plugin_id.clone(),
            level: LogLevel::Info,
            message: "loaded catalog".into(),
        };

        let json = serde_json::to_string(&log_event).expect("serialize PluginLog");
        assert!(json.contains(r#""type":"plugin_log""#));
        assert!(json.contains(r#""pluginId":"anthology""#));

        let enriched = EngineEvent::MetadataEnriched {
            plugin_id,
            native_id: "/anime/frieren".into(),
            detail: Box::new(MediaDetail {
                native_id: "/anime/frieren".into(),
                plugin_id: None,
                media_type: MediaType::Series,
                is_anime: true,
                title: "Frieren".into(),
                original_title: None,
                year: Some(2023),
                synopsis: None,
                poster: None,
                backdrop: None,
                genres: Vec::new(),
                rating: None,
                status: None,
                content_rating: Default::default(),
                external_ids: Default::default(),
                seasons: None,
                episodes: Vec::new(),
                relations: Vec::new(),
            }),
        };

        let round_trip: EngineEvent = serde_json::from_str(
            &serde_json::to_string(&enriched).expect("serialize MetadataEnriched"),
        )
        .expect("deserialize MetadataEnriched");
        assert_eq!(round_trip, enriched);
    }
}
