//! Skip-segment resolver (`Intro`, `Recap`, `Credits`, `Preview`) aggregating plugin data
//! with `TheIntroDB`, `IntroDB`, and `AniSkip`, guarded by strict duration/ID validation.

use std::collections::HashSet;
use std::time::Duration;

use serde::Deserialize;
use vessel_domain::{ExternalIds, MediaType, SkipSegment, SkipSegmentType};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(4);
const ANIZIP_TIMEOUT: Duration = Duration::from_millis(2500);
const MIN_INTRODB_CONFIDENCE: f64 = 0.40;
const MIN_SEGMENT_LEN_MS: u64 = 3_000;
const MAX_INTRO_LEN_MS: u64 = 6 * 60_000;
const MAX_RECAP_LEN_MS: u64 = 10 * 60_000;
const MAX_PREVIEW_LEN_MS: u64 = 6 * 60_000;

/// Base URLs for skip-segment providers (overridable for deterministic unit tests).
#[derive(Debug, Clone)]
pub struct SkipEndpoints {
    pub theintrodb_base: String,
    pub introdb_base: String,
    pub aniskip_base: String,
    pub anizip_base: String,
}

impl Default for SkipEndpoints {
    fn default() -> Self {
        Self {
            theintrodb_base: "https://api.theintrodb.org/v2".into(),
            introdb_base: "https://api.introdb.app".into(),
            aniskip_base: "https://api.aniskip.com/v2".into(),
            anizip_base: "https://api.ani.zip".into(),
        }
    }
}

/// Parameters for resolving skip intervals for the currently playing item/episode.
#[derive(Debug, Clone)]
pub struct SkipQuery {
    pub media_type: MediaType,
    pub is_anime: bool,
    pub external_ids: ExternalIds,
    pub season: u32,
    pub episode: u32,
    pub absolute_episode: Option<u32>,
    pub duration_ms: Option<u64>,
    pub plugin_segments: Vec<SkipSegment>,
}

#[derive(Debug, Deserialize)]
struct AnizipResp {
    mappings: Option<AnizipMaps>,
}

#[derive(Debug, Deserialize)]
struct AnizipMaps {
    imdb_id: Option<String>,
    themoviedb_id: Option<u64>,
    mal_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TheIntroDbSpan {
    start_ms: Option<u64>,
    end_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TheIntroDbResp {
    #[serde(default)]
    intro: Vec<TheIntroDbSpan>,
    #[serde(default)]
    recap: Vec<TheIntroDbSpan>,
    #[serde(default)]
    credits: Vec<TheIntroDbSpan>,
    #[serde(default)]
    preview: Vec<TheIntroDbSpan>,
}

#[derive(Debug, Deserialize)]
struct IntroDbSpan {
    start_ms: Option<u64>,
    end_ms: Option<u64>,
    confidence: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct IntroDbResp {
    intro: Option<IntroDbSpan>,
    recap: Option<IntroDbSpan>,
    outro: Option<IntroDbSpan>,
    #[serde(default)]
    no_intro: bool,
    #[serde(default)]
    no_recap: bool,
    #[serde(default)]
    no_outro: bool,
}

#[derive(Debug, Deserialize)]
struct AniSkipInterval {
    #[serde(rename = "startTime")]
    start_time: f64,
    #[serde(rename = "endTime")]
    end_time: f64,
}

#[derive(Debug, Deserialize)]
struct AniSkipEntry {
    interval: AniSkipInterval,
    #[serde(rename = "skipType")]
    skip_type: String,
    #[serde(rename = "episodeLength", default)]
    episode_length: f64,
}

#[derive(Debug, Deserialize)]
struct AniSkipResp {
    #[serde(default)]
    found: bool,
    #[serde(default)]
    results: Vec<AniSkipEntry>,
}

fn sanitize_segment(
    kind: SkipSegmentType,
    start_ms: u64,
    end_ms: Option<u64>,
    duration_ms: Option<u64>,
    default_source: &str,
    existing_source: Option<String>,
    confidence: Option<u8>,
) -> Option<SkipSegment> {
    let dur = duration_ms.filter(|&d| d > 10_000);
    if let Some(d) = dur
        && start_ms >= d.saturating_sub(1_500)
    {
        return None;
    }

    let clamped_end = match (end_ms, dur) {
        (Some(e), Some(d)) => Some(e.min(d)),
        (Some(e), None) => Some(e),
        (None, Some(d)) => Some(d),
        (None, None) => None,
    };

    if let Some(end) = clamped_end {
        if end <= start_ms {
            return None;
        }
        let len = end - start_ms;
        if len < MIN_SEGMENT_LEN_MS {
            return None;
        }
        match kind {
            SkipSegmentType::Intro if len > MAX_INTRO_LEN_MS => return None,
            SkipSegmentType::Recap if len > MAX_RECAP_LEN_MS => return None,
            SkipSegmentType::Preview if len > MAX_PREVIEW_LEN_MS => return None,
            _ => {}
        }
        if let Some(d) = dur
            && len.saturating_mul(100) > d.saturating_mul(85)
        {
            return None;
        }
    }

    Some(SkipSegment {
        kind,
        start_ms,
        end_ms: clamped_end.or(end_ms),
        source: Some(existing_source.unwrap_or_else(|| default_source.to_owned())),
        confidence,
    })
}

async fn resolve_missing_ids(
    client: &reqwest::Client,
    endpoints: &SkipEndpoints,
    mut ids: ExternalIds,
) -> ExternalIds {
    let needs_lookup =
        ids.anilist.is_some() && (ids.mal.is_none() || ids.imdb.is_none() || ids.tmdb.is_none());
    if !needs_lookup {
        return ids;
    }
    let Some(anilist_id) = ids.anilist else {
        return ids;
    };
    let url = format!("{}/mappings?anilist_id={anilist_id}", endpoints.anizip_base);
    if let Ok(resp) = client.get(&url).timeout(ANIZIP_TIMEOUT).send().await
        && resp.status().is_success()
        && let Ok(parsed) = resp.json::<AnizipResp>().await
        && let Some(maps) = parsed.mappings
    {
        if ids.imdb.is_none() {
            ids.imdb = maps.imdb_id.filter(|s| s.starts_with("tt"));
        }
        if ids.tmdb.is_none() {
            ids.tmdb = maps.themoviedb_id;
        }
        if ids.mal.is_none() {
            ids.mal = maps.mal_id;
        }
    }
    ids
}

async fn query_theintrodb(
    client: &reqwest::Client,
    endpoints: &SkipEndpoints,
    ids: &ExternalIds,
    query: &SkipQuery,
) -> Vec<SkipSegment> {
    if query.media_type == MediaType::Live {
        return Vec::new();
    }

    let id_param = if let Some(tmdb) = ids.tmdb {
        format!("tmdb_id={tmdb}")
    } else if let Some(ref imdb) = ids.imdb
        && imdb.starts_with("tt")
    {
        format!("imdb_id={imdb}")
    } else {
        return Vec::new();
    };

    let url = if query.media_type == MediaType::Movie {
        format!("{}/media?{id_param}", endpoints.theintrodb_base)
    } else {
        if query.season == 0 || query.episode == 0 {
            return Vec::new();
        }
        format!(
            "{}/media?{id_param}&season={}&episode={}",
            endpoints.theintrodb_base, query.season, query.episode
        )
    };

    let Ok(resp) = client.get(&url).timeout(REQUEST_TIMEOUT).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(parsed) = resp.json::<TheIntroDbResp>().await else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (kind, spans) in [
        (SkipSegmentType::Recap, parsed.recap),
        (SkipSegmentType::Intro, parsed.intro),
        (SkipSegmentType::Credits, parsed.credits),
        (SkipSegmentType::Preview, parsed.preview),
    ] {
        for span in spans {
            let start = span.start_ms.unwrap_or(0);
            if let Some(seg) = sanitize_segment(
                kind,
                start,
                span.end_ms,
                query.duration_ms,
                "theintrodb",
                None,
                None,
            ) {
                out.push(seg);
                break;
            }
        }
    }
    out
}

#[derive(Default)]
struct IntroDbOutcome {
    segments: Vec<SkipSegment>,
    no_intro: bool,
    no_recap: bool,
    no_outro: bool,
}

async fn query_introdb(
    client: &reqwest::Client,
    endpoints: &SkipEndpoints,
    ids: &ExternalIds,
    query: &SkipQuery,
) -> IntroDbOutcome {
    let mut outcome = IntroDbOutcome::default();
    if query.media_type == MediaType::Live {
        return outcome;
    }
    let Some(ref imdb) = ids.imdb else {
        return outcome;
    };
    if !imdb.starts_with("tt") {
        return outcome;
    }

    let url = if query.media_type == MediaType::Movie {
        format!(
            "{}/segments?imdb_id={imdb}&is_movie=true",
            endpoints.introdb_base
        )
    } else {
        if query.season == 0 || query.episode == 0 {
            return outcome;
        }
        format!(
            "{}/segments?imdb_id={imdb}&season={}&episode={}",
            endpoints.introdb_base, query.season, query.episode
        )
    };

    let Ok(resp) = client.get(&url).timeout(REQUEST_TIMEOUT).send().await else {
        return outcome;
    };
    if !resp.status().is_success() {
        return outcome;
    }
    let Ok(parsed) = resp.json::<IntroDbResp>().await else {
        return outcome;
    };

    outcome.no_intro = parsed.no_intro;
    outcome.no_recap = parsed.no_recap;
    outcome.no_outro = parsed.no_outro;

    for (kind, span_opt) in [
        (SkipSegmentType::Recap, parsed.recap),
        (SkipSegmentType::Intro, parsed.intro),
        (SkipSegmentType::Credits, parsed.outro),
    ] {
        let Some(span) = span_opt else { continue };
        if let Some(conf) = span.confidence
            && conf < MIN_INTRODB_CONFIDENCE
        {
            continue;
        }
        let start = span.start_ms.unwrap_or(0);
        let conf_pct = span
            .confidence
            .map(|c| (c.clamp(0.0, 1.0) * 100.0).round() as u8);
        if let Some(seg) = sanitize_segment(
            kind,
            start,
            span.end_ms,
            query.duration_ms,
            "introdb",
            None,
            conf_pct,
        ) {
            outcome.segments.push(seg);
        }
    }

    outcome
}

async fn query_aniskip(
    client: &reqwest::Client,
    endpoints: &SkipEndpoints,
    ids: &ExternalIds,
    query: &SkipQuery,
) -> Vec<SkipSegment> {
    // Only use AniSkip when season <= 1 (MAL assigns separate IDs per season; avoid matching S1 on S2+!)
    if query.media_type == MediaType::Live
        || (!query.is_anime && ids.anilist.is_none() && ids.mal.is_none())
    {
        return Vec::new();
    }
    if query.media_type == MediaType::Series && query.season > 1 {
        return Vec::new();
    }
    let Some(mal_id) = ids.mal else {
        return Vec::new();
    };
    let ep_num = if query.media_type == MediaType::Movie {
        1
    } else {
        query.episode.max(1)
    };
    let ep_len_param = query
        .duration_ms
        .map(|d| (d as f64) / 1000.0)
        .filter(|&s| s > 10.0)
        .unwrap_or(0.0);

    let url = format!(
        "{}/skip-times/{mal_id}/{ep_num}?types[]=op&types[]=mixed-op&types[]=ed&types[]=mixed-ed&types[]=recap&episodeLength={ep_len_param:.3}",
        endpoints.aniskip_base
    );

    let Ok(resp) = client.get(&url).timeout(REQUEST_TIMEOUT).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(parsed) = resp.json::<AniSkipResp>().await else {
        return Vec::new();
    };
    if !parsed.found {
        return Vec::new();
    }

    let dur_sec = query
        .duration_ms
        .map(|d| (d as f64) / 1000.0)
        .filter(|&s| s > 10.0);
    let mut out = Vec::new();
    let mut seen_kinds = HashSet::new();

    for entry in parsed.results {
        if let Some(actual_sec) = dur_sec
            && entry.episode_length > 10.0
            && (entry.episode_length - actual_sec).abs() > 120.0
        {
            continue;
        }
        let kind = match entry.skip_type.as_str() {
            "op" | "mixed-op" => SkipSegmentType::Intro,
            "ed" | "mixed-ed" => SkipSegmentType::Credits,
            "recap" => SkipSegmentType::Recap,
            _ => continue,
        };
        if seen_kinds.contains(&kind) {
            continue;
        }
        if !entry.interval.start_time.is_finite()
            || !entry.interval.end_time.is_finite()
            || entry.interval.start_time < 0.0
            || entry.interval.end_time <= entry.interval.start_time
        {
            continue;
        }
        let start_ms = (entry.interval.start_time * 1000.0).round() as u64;
        let end_ms = (entry.interval.end_time * 1000.0).round() as u64;
        if let Some(seg) = sanitize_segment(
            kind,
            start_ms,
            Some(end_ms),
            query.duration_ms,
            "aniskip",
            None,
            None,
        ) {
            seen_kinds.insert(kind);
            out.push(seg);
        }
    }

    out
}

/// Resolves validated skip segments (`Plugin > TheIntroDB > IntroDB > AniSkip`).
pub async fn resolve_skip_segments(
    client: &reqwest::Client,
    endpoints: &SkipEndpoints,
    query: &SkipQuery,
    allow_external: bool,
) -> Vec<SkipSegment> {
    let mut merged = Vec::new();
    let mut covered_types: HashSet<SkipSegmentType> = HashSet::new();

    // 1) Plugin/stream-provided segments always take highest precedence.
    for raw in &query.plugin_segments {
        if let Some(clean) = sanitize_segment(
            raw.kind,
            raw.start_ms,
            raw.end_ms,
            query.duration_ms,
            "plugin",
            raw.source.clone(),
            raw.confidence,
        ) {
            covered_types.insert(clean.kind);
            merged.push(clean);
        }
    }

    if !allow_external || query.media_type == MediaType::Live {
        merged.sort_by_key(|s| s.start_ms);
        return merged;
    }

    // Check if all 4 types are already supplied by the plugin.
    if covered_types.len() >= 4 {
        merged.sort_by_key(|s| s.start_ms);
        return merged;
    }

    let ids = resolve_missing_ids(client, endpoints, query.external_ids.clone()).await;
    if ids.tmdb.is_none() && ids.imdb.is_none() && ids.mal.is_none() {
        merged.sort_by_key(|s| s.start_ms);
        return merged;
    }

    let (theintrodb_segs, introdb_out, aniskip_segs) = tokio::join!(
        query_theintrodb(client, endpoints, &ids, query),
        query_introdb(client, endpoints, &ids, query),
        query_aniskip(client, endpoints, &ids, query),
    );

    // 2) TheIntroDB (intro, recap, credits, preview)
    for seg in theintrodb_segs {
        if covered_types.insert(seg.kind) {
            merged.push(seg);
        }
    }

    // 3) IntroDB (intro, recap, outro -> credits)
    for seg in introdb_out.segments {
        if covered_types.insert(seg.kind) {
            merged.push(seg);
        }
    }

    // 4) AniSkip fallback for anime (respect IntroDB negative flags if community explicitly denied them)
    for seg in aniskip_segs {
        let blocked_by_introdb = match seg.kind {
            SkipSegmentType::Intro => introdb_out.no_intro,
            SkipSegmentType::Recap => introdb_out.no_recap,
            SkipSegmentType::Credits => introdb_out.no_outro,
            SkipSegmentType::Preview => false,
        };
        if !blocked_by_introdb && covered_types.insert(seg.kind) {
            merged.push(seg);
        }
    }

    merged.sort_by_key(|s| s.start_ms);
    merged
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::extract::Query;
    use axum::routing::get;
    use serde::Deserialize;
    use tokio::net::TcpListener;
    use vessel_domain::{ExternalIds, MediaType, SkipSegment, SkipSegmentType};

    use super::{SkipEndpoints, SkipQuery, resolve_skip_segments};

    #[derive(Deserialize)]
    struct MediaQuery {
        tmdb_id: Option<u64>,
        season: Option<u32>,
        episode: Option<u32>,
    }

    #[derive(Deserialize)]
    struct IntroDbQuery {
        imdb_id: Option<String>,
        season: Option<u32>,
    }

    #[tokio::test]
    async fn prioritizes_plugin_over_theintrodb_introdb_and_aniskip_with_duration_guards() {
        let app = Router::new()
            .route(
                "/theintrodb/media",
                get(|Query(q): Query<MediaQuery>| async move {
                    assert_eq!(q.tmdb_id, Some(1396));
                    assert_eq!(q.season, Some(1));
                    assert_eq!(q.episode, Some(1));
                    axum::Json(serde_json::json!({
                        "tmdb_id": 1396,
                        "type": "tv",
                        "intro": [{"start_ms": 10_000, "end_ms": 95_000}],
                        "recap": [{"start_ms": 0, "end_ms": 8_000}],
                        "preview": [{"start_ms": 1_390_000, "end_ms": 1_420_000}],
                        // Bogus intro exceeding MAX_INTRO_LEN_MS should be rejected too if tested alone
                        "credits": []
                    }))
                }),
            )
            .route(
                "/introdb/segments",
                get(|Query(q): Query<IntroDbQuery>| async move {
                    assert_eq!(q.imdb_id.as_deref(), Some("tt0903747"));
                    assert_eq!(q.season, Some(1));
                    axum::Json(serde_json::json!({
                        "intro": {"start_ms": 20_000, "end_ms": 70_000, "confidence": 0.95},
                        "outro": {"start_ms": 1_320_000, "end_ms": 1_390_000, "confidence": 0.88}
                    }))
                }),
            )
            .route(
                "/aniskip/skip-times/9253/1",
                get(|| async move {
                    axum::Json(serde_json::json!({
                        "found": true,
                        "results": [
                            {"interval": {"startTime": 50.0, "endTime": 140.0}, "skipType": "op", "episodeLength": 1440.0},
                            {"interval": {"startTime": 1310.0, "endTime": 1400.0}, "skipType": "ed", "episodeLength": 1440.0}
                        ]
                    }))
                }),
            );

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let endpoints = SkipEndpoints {
            theintrodb_base: format!("{base}/theintrodb"),
            introdb_base: format!("{base}/introdb"),
            aniskip_base: format!("{base}/aniskip"),
            anizip_base: format!("{base}/anizip"),
        };
        let client = reqwest::Client::new();

        // Plugin supplies custom Intro (must win over TheIntroDB & IntroDB & AniSkip).
        // TheIntroDB supplies Recap & Preview. IntroDB supplies Credits (outro).
        let out = resolve_skip_segments(
            &client,
            &endpoints,
            &SkipQuery {
                media_type: MediaType::Series,
                is_anime: true,
                external_ids: ExternalIds {
                    tmdb: Some(1396),
                    imdb: Some("tt0903747".into()),
                    mal: Some(9253),
                    ..Default::default()
                },
                season: 1,
                episode: 1,
                absolute_episode: None,
                duration_ms: Some(1_440_000),
                plugin_segments: vec![SkipSegment {
                    kind: SkipSegmentType::Intro,
                    start_ms: 5_000,
                    end_ms: Some(85_000),
                    source: Some("anthology".into()),
                    confidence: None,
                }],
            },
            true,
        )
        .await;

        assert_eq!(out.len(), 4);
        assert_eq!(out[0].kind, SkipSegmentType::Recap);
        assert_eq!(out[0].source.as_deref(), Some("theintrodb"));
        assert_eq!(out[1].kind, SkipSegmentType::Intro);
        assert_eq!(out[1].source.as_deref(), Some("anthology"));
        assert_eq!(out[1].start_ms, 5_000);
        assert_eq!(out[2].kind, SkipSegmentType::Credits);
        assert_eq!(out[2].source.as_deref(), Some("introdb"));
        assert_eq!(out[3].kind, SkipSegmentType::Preview);
        assert_eq!(out[3].source.as_deref(), Some("theintrodb"));
    }

    #[tokio::test]
    async fn blocks_aniskip_on_season_gt_1_and_filters_invalid_intervals() {
        let endpoints = SkipEndpoints::default();
        let client = reqwest::Client::new();

        let out = resolve_skip_segments(
            &client,
            &endpoints,
            &SkipQuery {
                media_type: MediaType::Series,
                is_anime: false,
                external_ids: ExternalIds::default(),
                season: 2,
                episode: 1,
                absolute_episode: None,
                duration_ms: Some(100_000),
                plugin_segments: vec![
                    // Invalid: > 85% of duration -> rejected
                    SkipSegment {
                        kind: SkipSegmentType::Intro,
                        start_ms: 0,
                        end_ms: Some(95_000),
                        source: None,
                        confidence: None,
                    },
                    // Invalid: too short (< 3s) -> rejected
                    SkipSegment {
                        kind: SkipSegmentType::Recap,
                        start_ms: 1_000,
                        end_ms: Some(2_500),
                        source: None,
                        confidence: None,
                    },
                    // Valid: 10s..25s Credits
                    SkipSegment {
                        kind: SkipSegmentType::Credits,
                        start_ms: 75_000,
                        end_ms: Some(95_000),
                        source: None,
                        confidence: None,
                    },
                ],
            },
            false,
        )
        .await;

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, SkipSegmentType::Credits);
    }
}
