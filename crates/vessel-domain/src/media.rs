//! Media catalog, detail, episode, and watch progress models.

use serde::{Deserialize, Serialize};

use crate::ids::{ExternalIds, MediaRef, PluginId};

/// Structural playback format of a media item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Movie,
    Series,
    Live,
}

/// Audio/subtitle distribution tags attached to an episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeTag {
    Sub,
    Dub,
    Raw,
}

/// Age/sensitivity classification used for filtering and poster blurring.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentRating {
    #[default]
    Sfw,
    Nsfw,
}

/// Relationship type between media entries (such as anime sequels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Sequel,
    Prequel,
    SideStory,
    SpinOff,
    Related,
}

/// Lightweight card item returned by home feeds and search results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSummary {
    pub native_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<PluginId>,
    #[serde(rename = "type")]
    pub media_type: MediaType,
    #[serde(default)]
    pub is_anime: bool,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f32>,
}

/// Optional lazy-loaded season descriptor for long-running series.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeasonSummary {
    pub id: String,
    pub season: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
}

/// Related entry displayed on a detail page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaRelation {
    pub kind: RelationKind,
    pub native_id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub media_type: MediaType,
    #[serde(default)]
    pub is_anime: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
}

/// Single playable episode or virtual movie entry owned by a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub stream_id: String,
    pub season: u32,
    pub episode: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub absolute_episode: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub air_date: Option<String>,
    #[serde(default)]
    pub tags: Vec<EpisodeTag>,
}

/// Full media detail returned by a provider and optionally enriched by core metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDetail {
    pub native_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<PluginId>,
    #[serde(rename = "type")]
    pub media_type: MediaType,
    #[serde(default)]
    pub is_anime: bool,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default)]
    pub content_rating: ContentRating,
    #[serde(default)]
    pub external_ids: ExternalIds,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seasons: Option<Vec<SeasonSummary>>,
    #[serde(default)]
    pub episodes: Vec<Episode>,
    #[serde(default)]
    pub relations: Vec<MediaRelation>,
}

/// Dual-coordinate watch progress preserving both `(season, episode)` and `absolute_episode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackProgress {
    pub media: MediaRef,
    pub stream_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_id: Option<String>,
    pub season: u32,
    pub episode: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub absolute_episode: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backdrop_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode_title: Option<String>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub completed: bool,
    pub last_watched_at: String,
}

/// User-selected cross-plugin stream source linked to a detail page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedMediaSource {
    pub plugin_id: PluginId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    pub native_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
}

impl PlaybackProgress {
    /// Returns watched fraction in `[0.0, 1.0]`, returning `0.0` when duration is zero.
    pub fn completion_ratio(&self) -> f64 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        (self.position_ms as f64 / self.duration_ms as f64).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentRating, MediaDetail, MediaSummary, MediaType, PlaybackProgress};
    use crate::{MediaRef, PluginId};

    #[test]
    fn deserializes_minimal_plugin_detail_with_defaults() {
        let raw = r#"{
            "nativeId": "/anime/gintama",
            "type": "series",
            "isAnime": true,
            "title": "Gintama",
            "episodes": [
                {
                    "streamId": "/watch/gintama-50",
                    "season": 1,
                    "episode": 50,
                    "absoluteEpisode": 50
                }
            ]
        }"#;

        let detail: MediaDetail = serde_json::from_str(raw).expect("parse minimal MediaDetail");
        assert_eq!(detail.media_type, MediaType::Series);
        assert!(detail.is_anime);
        assert_eq!(detail.content_rating, ContentRating::Sfw);
        assert!(detail.plugin_id.is_none());
        assert!(detail.external_ids.is_empty());
        assert_eq!(detail.episodes.len(), 1);
        assert_eq!(detail.episodes[0].absolute_episode, Some(50));
        assert!(detail.episodes[0].tags.is_empty());
    }

    #[test]
    fn rejects_non_structural_anime_media_type() {
        let raw = r#"{
            "nativeId": "/anime/gintama",
            "type": "anime",
            "title": "Gintama"
        }"#;

        assert!(serde_json::from_str::<MediaSummary>(raw).is_err());
    }

    #[test]
    fn computes_completion_ratio_safely() {
        let plugin_id = PluginId::new("anthology").expect("valid plugin id");
        let media = MediaRef::new(plugin_id, "/anime/frieren").expect("valid media ref");
        let mut progress = PlaybackProgress {
            media,
            stream_id: "/watch/frieren-12".into(),
            canonical_id: Some("anilist:154587".into()),
            season: 1,
            episode: 12,
            absolute_episode: Some(12),
            title: None,
            poster_url: None,
            backdrop_url: None,
            episode_title: None,
            position_ms: 0,
            duration_ms: 0,
            completed: false,
            last_watched_at: "2026-11-18T13:00:00Z".into(),
        };

        assert_eq!(progress.completion_ratio(), 0.0);

        progress.duration_ms = 1_000;
        progress.position_ms = 750;
        assert!((progress.completion_ratio() - 0.75).abs() < f64::EPSILON);

        progress.position_ms = 2_000;
        assert_eq!(progress.completion_ratio(), 1.0);
    }
}
