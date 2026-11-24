//! Stream candidates, audio priorities, and subtitle tracks.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Transport or container type of a resolved or embed stream candidate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    Hls,
    Dash,
    MpegTs,
    Mp4,
    Mkv,
    Webm,
    #[default]
    Auto,
    Embed,
}

/// Spoken audio classification of a stream candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioType {
    Sub,
    Dub,
    Original,
}

/// User preference for ordering stream candidates by audio classification.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioPriority {
    #[default]
    Auto,
    SubFirst,
    DubFirst,
    OriginalFirst,
}

/// Capability and routing flags attached to a stream candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamFlag {
    Subtitled,
    Dubbed,
    MultiAudio,
    Hdr,
    Direct,
    RequiresProxy,
}

/// Supported external or inline subtitle file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleFormat {
    Srt,
    Ass,
    Ssa,
    Vtt,
}

/// Location or inline payload of a subtitle track (prevents empty/ambiguous states).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubtitleSource {
    Url {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
    InlineContent {
        content: String,
    },
}

/// Single subtitle track provided alongside a stream or resolved independently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrack {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub source: SubtitleSource,
    pub lang: String,
    pub label: String,
    pub format: SubtitleFormat,
    #[serde(default)]
    pub is_default: bool,
}

/// Category of a skippable timeline interval (`intro`, `recap`, `credits`, `preview`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipSegmentType {
    #[serde(alias = "op", alias = "opening")]
    Intro,
    Recap,
    #[serde(alias = "outro", alias = "ed", alias = "ending")]
    Credits,
    Preview,
}

/// Time-bounded segment inside a stream/episode that the player can offer to skip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkipSegment {
    #[serde(rename = "type", alias = "kind", alias = "skipType")]
    pub kind: SkipSegmentType,
    #[serde(alias = "start_ms", alias = "startTimeMs")]
    pub start_ms: u64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "end_ms",
        alias = "endTimeMs"
    )]
    pub end_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
}

/// Single playable or embed stream candidate returned by a provider or extractor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamCandidate {
    pub url: String,
    pub kind: StreamKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_type: Option<AudioType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub flags: Vec<StreamFlag>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub needs_extractor: bool,
    #[serde(default)]
    pub subtitles: Vec<SubtitleTrack>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "segments")]
    pub skip_segments: Vec<SkipSegment>,
}

/// Combined stream candidates and episode-level subtitle tracks from `getStreams`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamResult {
    #[serde(default)]
    pub streams: Vec<StreamCandidate>,
    #[serde(default)]
    pub subtitles: Vec<SubtitleTrack>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "segments")]
    pub skip_segments: Vec<SkipSegment>,
}

#[cfg(test)]
mod tests {
    use super::{
        AudioType, StreamFlag, StreamKind, StreamResult, SubtitleFormat, SubtitleSource,
        SubtitleTrack,
    };

    #[test]
    fn round_trips_discriminated_subtitle_sources() {
        let url_track = SubtitleTrack {
            id: Some("tr-ass".into()),
            source: SubtitleSource::Url {
                url: "https://cdn.anthology.example/subs/ep1.ass".into(),
                headers: [("Referer".into(), "https://anthology.example".into())].into(),
            },
            lang: "tr".into(),
            label: "Turkish (ASS)".into(),
            format: SubtitleFormat::Ass,
            is_default: true,
        };

        let json = serde_json::to_string(&url_track).expect("serialize url subtitle");
        assert!(json.contains(r#""kind":"url""#));
        let parsed: SubtitleTrack = serde_json::from_str(&json).expect("deserialize url subtitle");
        assert_eq!(parsed, url_track);

        let inline_raw = r#"{
            "source": { "kind": "inline_content", "content": "1\n00:00:01,000 --> 00:00:03,000\nHi" },
            "lang": "en",
            "label": "English",
            "format": "srt"
        }"#;
        let inline_track: SubtitleTrack =
            serde_json::from_str(inline_raw).expect("parse inline subtitle");
        assert!(!inline_track.is_default);
        assert!(matches!(
            inline_track.source,
            SubtitleSource::InlineContent { .. }
        ));
    }

    #[test]
    fn deserializes_stream_result_with_headers_and_flags() {
        let raw = r#"{
            "streams": [
                {
                    "url": "https://cdn.anthology.example/master.m3u8",
                    "kind": "hls",
                    "quality": "1080p",
                    "label": "Anthology Main",
                    "language": "ja",
                    "audioType": "sub",
                    "flags": ["subtitled", "requires_proxy"],
                    "headers": {
                        "Referer": "https://anthology.example/",
                        "User-Agent": "Mozilla/5.0"
                    }
                }
            ]
        }"#;

        let result: StreamResult = serde_json::from_str(raw).expect("parse StreamResult");
        assert_eq!(result.streams.len(), 1);
        assert!(result.subtitles.is_empty());

        let candidate = &result.streams[0];
        assert_eq!(candidate.kind, StreamKind::Hls);
        assert_eq!(candidate.audio_type, Some(AudioType::Sub));
        assert_eq!(
            candidate.flags,
            vec![StreamFlag::Subtitled, StreamFlag::RequiresProxy]
        );
        assert_eq!(
            candidate.headers.get("Referer").map(String::as_str),
            Some("https://anthology.example/")
        );

        let ts_kind: StreamKind = serde_json::from_str(r#""mpeg_ts""#).expect("parse mpeg_ts");
        let webm_kind: StreamKind = serde_json::from_str(r#""webm""#).expect("parse webm");
        let auto_kind: StreamKind = serde_json::from_str(r#""auto""#).expect("parse auto");
        assert_eq!(ts_kind, StreamKind::MpegTs);
        assert_eq!(webm_kind, StreamKind::Webm);
        assert_eq!(auto_kind, StreamKind::Auto);
    }
}
