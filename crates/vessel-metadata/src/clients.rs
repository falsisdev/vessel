//! Metadata HTTP clients (AniList GraphQL, TMDB v3, keyless Cinemeta fallback) & fuzzy matching.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use vessel_domain::{ExternalIds, MediaDetail, MediaType, VesselError, VesselResult};

use crate::coordinates::{CanonicalEpisodeMeta, SeasonSpan, season_episode_to_absolute};
use crate::merge::{CanonicalMetadata, canonical_id_from_external_ids};

/// Minimum similarity confidence required for heuristic matching without explicit `ExternalIds`.
pub const MIN_MATCH_CONFIDENCE: f32 = 0.65;

const METADATA_TIMEOUT: Duration = Duration::from_secs(10);

/// Configurable upstream metadata API base URLs (overridable in tests).
#[derive(Debug, Clone)]
pub struct MetaEndpoints {
    pub anilist_graphql: String,
    pub tmdb_api_base: String,
    pub tmdb_image_base: String,
    pub cinemeta_base: String,
    pub jikan_base: String,
    pub kitsu_base: String,
    pub simkl_base: String,
    pub imdb_suggest_base: String,
}

impl Default for MetaEndpoints {
    fn default() -> Self {
        Self {
            anilist_graphql: "https://graphql.anilist.co".into(),
            tmdb_api_base: "https://api.themoviedb.org/3".into(),
            tmdb_image_base: "https://image.tmdb.org/t/p".into(),
            cinemeta_base: "https://v3-cinemeta.strem.io".into(),
            jikan_base: "https://api.jikan.moe/v4".into(),
            kitsu_base: "https://kitsu.io/api/edge".into(),
            simkl_base: "https://api.simkl.com".into(),
            imdb_suggest_base: "https://v3.sg.media-imdb.com/suggestion".into(),
        }
    }
}

/// Filter selecting which metadata database(s) interactive Fix Match search queries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataProviderFilter {
    #[default]
    All,
    Tmdb,
    Imdb,
    Simkl,
    Anilist,
    Mal,
    Kitsu,
}

/// Interactive search result shown in the Fix Match picker across providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSearchHit {
    pub canonical_id: String,
    pub provider: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,
    pub external_ids: ExternalIds,
}

/// Combined search hits and non-fatal provider guidance notes (e.g. missing optional API keys).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSearchResponse {
    pub hits: Vec<MetadataSearchHit>,
    #[serde(default)]
    pub notes: Vec<String>,
}

fn clean_search_title(input: &str) -> String {
    let mut s = input.trim().to_string();
    // Strip trailing/inline bracketed and parenthesized annotations such as [TR], (2024), (Dub).
    for (open, close) in [('(', ')'), ('[', ']')] {
        while let Some(start) = s.find(open) {
            let Some(end_rel) = s[start..].find(close) else {
                break;
            };
            let inner = &s[start + 1..start + end_rel];
            let inner_lc = inner.trim().to_ascii_lowercase();
            let is_noise = inner_lc.len() <= 4 && inner_lc.chars().all(|c| c.is_ascii_digit())
                || [
                    "tr", "en", "sub", "dub", "dublaj", "altyazi", "altyazı", "hd", "fhd", "1080p",
                    "720p", "4k", "tv", "ova",
                ]
                .iter()
                .any(|tag| inner_lc.contains(tag));
            if is_noise || start > 0 {
                s.replace_range(start..=start + end_rel, " ");
            } else {
                break;
            }
        }
    }

    let noise_words = [
        "1080p",
        "720p",
        "480p",
        "2160p",
        "4k",
        "hdr",
        "bluray",
        "webrip",
        "web-dl",
        "türkçe",
        "turkce",
        "dublaj",
        "altyazı",
        "altyazi",
        "altyazılı",
        "izle",
        "sezon",
        "season",
        "part",
        "cour",
    ];
    let tokens: Vec<&str> = s
        .split_whitespace()
        .filter(|tok| {
            let lc = tok
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase();
            !lc.is_empty() && !noise_words.contains(&lc.as_str())
        })
        .collect();
    let cleaned = tokens.join(" ");
    if cleaned.is_empty() {
        input.trim().to_owned()
    } else {
        cleaned
    }
}

fn encode_path_segment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        let ch = *byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            out.push(ch);
        } else {
            out.push_str(&format!("%{:02X}", byte));
        }
    }
    out
}

fn normalize_tokens(input: &str) -> BTreeSet<String> {
    input
        .to_ascii_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|tok| !tok.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Computes normalized token-overlap + year proximity confidence in `[0.0, 1.0]`.
pub fn title_match_confidence(
    query_title: &str,
    query_year: Option<u16>,
    candidate_titles: &[&str],
    candidate_year: Option<u16>,
) -> f32 {
    let cleaned_query = clean_search_title(query_title);
    let q_tokens_raw = normalize_tokens(query_title);
    let q_tokens_clean = normalize_tokens(&cleaned_query);
    if q_tokens_raw.is_empty() && q_tokens_clean.is_empty() {
        return 0.0;
    }

    let mut best_text_score = 0.0_f32;
    for candidate in candidate_titles {
        let c_tokens = normalize_tokens(candidate);
        if c_tokens.is_empty() {
            continue;
        }
        for q_tokens in [&q_tokens_raw, &q_tokens_clean] {
            if q_tokens.is_empty() {
                continue;
            }
            let intersection = q_tokens.intersection(&c_tokens).count() as f32;
            let union = q_tokens.union(&c_tokens).count() as f32;
            let jaccard = if union > 0.0 {
                intersection / union
            } else {
                0.0
            };
            best_text_score = best_text_score.max(jaccard);
            // Boost clean subset containment when query is slightly noisier or shorter than canonical title.
            if q_tokens.len() >= 2 && q_tokens.is_subset(&c_tokens) {
                let ratio = (q_tokens.len() as f32 / c_tokens.len() as f32).max(0.72);
                best_text_score = best_text_score.max(ratio);
            }
        }
    }

    match (query_year, candidate_year) {
        (Some(qy), Some(cy)) => {
            let diff = qy.abs_diff(cy);
            if diff <= 1 {
                (best_text_score + 0.15).min(1.0)
            } else if diff > 3 {
                (best_text_score - 0.30).max(0.0)
            } else {
                best_text_score
            }
        }
        _ => best_text_score,
    }
}

fn map_tmdb_lang(preferred_lang: &str) -> String {
    let clean = preferred_lang.trim();
    if clean.contains('-') {
        clean.to_owned()
    } else if clean.eq_ignore_ascii_case("tr") {
        "tr-TR".into()
    } else if clean.eq_ignore_ascii_case("ja") {
        "ja-JP".into()
    } else {
        "en-US".into()
    }
}

#[derive(Serialize)]
struct AniListRequest {
    query: &'static str,
    variables: serde_json::Value,
}

#[derive(Deserialize)]
struct AniListResponse {
    data: Option<AniListData>,
}

#[derive(Deserialize)]
struct AniListData {
    #[serde(rename = "Media")]
    media: Option<AniListMedia>,
    #[serde(rename = "Page")]
    page: Option<AniListPage>,
}

#[derive(Deserialize)]
struct AniListPage {
    #[serde(default)]
    media: Vec<AniListMedia>,
}

fn youtube_watch_url(id_or_url: &str) -> Option<String> {
    let trimmed = id_or_url.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Some(trimmed.to_owned());
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && (6..=20).contains(&trimmed.len())
    {
        return Some(format!("https://www.youtube.com/watch?v={trimmed}"));
    }
    None
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AniListMedia {
    id: u64,
    id_mal: Option<u64>,
    title: Option<AniListTitle>,
    description: Option<String>,
    season_year: Option<u16>,
    average_score: Option<f32>,
    genres: Option<Vec<String>>,
    cover_image: Option<AniListCover>,
    banner_image: Option<String>,
    episodes: Option<u32>,
    trailer: Option<AniListTrailer>,
}

#[derive(Deserialize)]
struct AniListTrailer {
    id: Option<String>,
    site: Option<String>,
}

#[derive(Deserialize)]
struct AniListTitle {
    romaji: Option<String>,
    english: Option<String>,
    native: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AniListCover {
    extra_large: Option<String>,
    large: Option<String>,
}

const ANILIST_BY_ID_QUERY: &str = r#"
query ($id: Int!) {
  Media(id: $id, type: ANIME) {
    id
    idMal
    title { romaji english native }
    description(asHtml: false)
    seasonYear
    averageScore
    genres
    coverImage { extraLarge large }
    bannerImage
    episodes
    trailer { id site }
  }
}
"#;

const ANILIST_SEARCH_QUERY: &str = r#"
query ($search: String!) {
  Page(page: 1, perPage: 8) {
    media(search: $search, type: ANIME, sort: [SEARCH_MATCH, POPULARITY_DESC]) {
      id
      idMal
      title { romaji english native }
      description(asHtml: false)
      seasonYear
      averageScore
      genres
      coverImage { extraLarge large }
      bannerImage
      episodes
      trailer { id site }
    }
  }
}
"#;

fn anilist_to_canonical(media: AniListMedia) -> CanonicalMetadata {
    let title_obj = media.title.as_ref();
    let title = title_obj
        .and_then(|t| t.english.clone().or_else(|| t.romaji.clone()))
        .unwrap_or_else(|| format!("AniList #{}", media.id));
    let original_title = title_obj.and_then(|t| t.native.clone().or_else(|| t.romaji.clone()));
    let poster = media
        .cover_image
        .as_ref()
        .and_then(|c| c.extra_large.clone().or_else(|| c.large.clone()));
    let trailer_url = media.trailer.and_then(|t| {
        if t.site.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("youtube")) {
            t.id.as_deref().and_then(youtube_watch_url)
        } else {
            None
        }
    });
    let external_ids = ExternalIds {
        anilist: Some(media.id),
        mal: media.id_mal,
        ..ExternalIds::default()
    };
    let season_spans = media
        .episodes
        .filter(|count| *count > 0)
        .map(|count| {
            vec![SeasonSpan {
                season: 1,
                episode_count: count,
            }]
        })
        .unwrap_or_default();

    CanonicalMetadata {
        canonical_id: format!("anilist:{}", media.id),
        external_ids,
        title,
        original_title,
        year: media.season_year,
        synopsis: media.description,
        synopsis_lang: Some("en".into()),
        poster,
        backdrop: media.banner_image,
        trailer_url,
        genres: media.genres.unwrap_or_default(),
        rating: media.average_score.map(|s| s / 10.0),
        season_spans,
        episodes: Vec::new(),
    }
}

#[derive(Deserialize)]
struct TmdbDetail {
    id: u64,
    title: Option<String>,
    name: Option<String>,
    original_title: Option<String>,
    original_name: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    vote_average: Option<f32>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    #[serde(default)]
    genres: Vec<TmdbGenre>,
    #[serde(default)]
    seasons: Vec<TmdbSeason>,
    external_ids: Option<TmdbExternalIds>,
    videos: Option<TmdbVideoList>,
}

#[derive(Deserialize)]
struct TmdbVideoList {
    #[serde(default)]
    results: Vec<TmdbVideo>,
}

#[derive(Deserialize)]
struct TmdbVideo {
    key: Option<String>,
    site: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    official: Option<bool>,
    iso_639_1: Option<String>,
}

#[derive(Deserialize)]
struct TmdbGenre {
    name: String,
}

#[derive(Deserialize)]
struct TmdbSeason {
    season_number: u32,
    episode_count: u32,
}

#[derive(Deserialize)]
struct TmdbExternalIds {
    imdb_id: Option<String>,
    tvdb_id: Option<u64>,
}

#[derive(Deserialize)]
struct TmdbSearchResponse {
    #[serde(default)]
    results: Vec<TmdbSearchItem>,
}

#[derive(Deserialize)]
struct TmdbSearchItem {
    id: u64,
    media_type: Option<String>,
    title: Option<String>,
    name: Option<String>,
    original_title: Option<String>,
    original_name: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
}

#[derive(Deserialize)]
struct JikanListResponse {
    #[serde(default)]
    data: Vec<JikanAnime>,
}

#[derive(Deserialize)]
struct JikanSingleResponse {
    data: Option<JikanAnime>,
}

#[derive(Deserialize)]
struct JikanAnime {
    mal_id: u64,
    title: Option<String>,
    title_english: Option<String>,
    title_japanese: Option<String>,
    synopsis: Option<String>,
    year: Option<u16>,
    score: Option<f32>,
    episodes: Option<u32>,
    aired: Option<JikanAired>,
    images: Option<JikanImages>,
    trailer: Option<JikanTrailer>,
    #[serde(default)]
    genres: Vec<JikanNamed>,
}

#[derive(Deserialize)]
struct JikanTrailer {
    youtube_id: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
struct JikanAired {
    from: Option<String>,
}

#[derive(Deserialize)]
struct JikanImages {
    jpg: Option<JikanImageSet>,
    webp: Option<JikanImageSet>,
}

#[derive(Deserialize)]
struct JikanImageSet {
    large_image_url: Option<String>,
    image_url: Option<String>,
}

#[derive(Deserialize)]
struct JikanNamed {
    name: String,
}

#[derive(Deserialize)]
struct KitsuListResponse {
    #[serde(default)]
    data: Vec<KitsuItem>,
}

#[derive(Deserialize)]
struct KitsuSingleResponse {
    data: Option<KitsuItem>,
}

#[derive(Deserialize)]
struct KitsuItem {
    id: String,
    attributes: KitsuAttributes,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KitsuAttributes {
    canonical_title: Option<String>,
    #[serde(default)]
    titles: BTreeMap<String, Option<String>>,
    synopsis: Option<String>,
    start_date: Option<String>,
    average_rating: Option<String>,
    episode_count: Option<u32>,
    poster_image: Option<KitsuImage>,
    cover_image: Option<KitsuImage>,
    youtube_video_id: Option<String>,
}

#[derive(Deserialize)]
struct KitsuImage {
    large: Option<String>,
    medium: Option<String>,
    original: Option<String>,
}

#[derive(Deserialize)]
struct ImdbSuggestEnvelope {
    #[serde(default)]
    d: Vec<ImdbSuggestItem>,
}

#[derive(Deserialize)]
struct ImdbSuggestItem {
    id: String,
    l: Option<String>,
    q: Option<String>,
    qid: Option<String>,
    y: Option<u16>,
    i: Option<ImdbSuggestImg>,
}

#[derive(Deserialize)]
struct ImdbSuggestImg {
    #[serde(rename = "imageUrl")]
    image_url: Option<String>,
}

#[derive(Deserialize)]
struct SimklSearchItem {
    title: Option<String>,
    year: Option<u16>,
    poster: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    ids: Option<SimklIds>,
}

#[derive(Deserialize)]
struct SimklIds {
    simkl: Option<u64>,
    simkl_id: Option<u64>,
    imdb: Option<String>,
    tmdb: Option<serde_json::Value>,
    mal: Option<serde_json::Value>,
    anilist: Option<serde_json::Value>,
    kitsu: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct SimklDetail {
    title: Option<String>,
    en_title: Option<String>,
    year: Option<u16>,
    overview: Option<String>,
    poster: Option<String>,
    fanart: Option<String>,
    total_episodes: Option<u32>,
    #[serde(default)]
    genres: Vec<String>,
    ratings: Option<SimklRatings>,
    ids: Option<SimklIds>,
}

#[derive(Deserialize)]
struct SimklRatings {
    simkl: Option<SimklRatingScore>,
}

#[derive(Deserialize)]
struct SimklRatingScore {
    rating: Option<f32>,
}

#[derive(Deserialize)]
struct TmdbSeasonDetail {
    #[serde(default)]
    episodes: Vec<TmdbEpisode>,
}

#[derive(Deserialize)]
struct TmdbEpisode {
    episode_number: u32,
    name: Option<String>,
    overview: Option<String>,
    still_path: Option<String>,
    air_date: Option<String>,
}

#[derive(Deserialize)]
struct CinemetaMetaEnvelope {
    meta: Option<CinemetaMeta>,
}

#[derive(Deserialize)]
struct CinemetaCatalogEnvelope {
    #[serde(default)]
    metas: Vec<CinemetaMeta>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CinemetaMeta {
    id: String,
    name: Option<String>,
    description: Option<String>,
    poster: Option<String>,
    background: Option<String>,
    #[serde(default)]
    genres: Vec<String>,
    imdb_rating: Option<String>,
    release_info: Option<String>,
    year: Option<serde_json::Value>,
    #[serde(default)]
    videos: Vec<CinemetaVideo>,
    #[serde(default)]
    trailers: Vec<CinemetaTrailer>,
    #[serde(default)]
    trailer_streams: Vec<CinemetaTrailerStream>,
}

#[derive(Deserialize)]
struct CinemetaTrailer {
    source: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CinemetaTrailerStream {
    yt_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CinemetaVideo {
    season: Option<u32>,
    episode: Option<u32>,
    name: Option<String>,
    title: Option<String>,
    overview: Option<String>,
    description: Option<String>,
    thumbnail: Option<String>,
    released: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AniZipResponse {
    #[serde(default)]
    episodes: BTreeMap<String, AniZipEpisode>,
    mappings: Option<AniZipMappings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AniZipMappings {
    imdb_id: Option<String>,
    themoviedb_id: Option<u64>,
    mal_id: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AniZipEpisode {
    season_number: Option<u32>,
    episode_number: Option<u32>,
    absolute_episode_number: Option<u32>,
    title: Option<BTreeMap<String, String>>,
    overview: Option<String>,
    image: Option<String>,
    air_date: Option<String>,
}

fn parse_year_prefix(s: Option<&str>) -> Option<u16> {
    s.and_then(|raw| raw.get(0..4)?.parse::<u16>().ok())
}

fn parse_cinemeta_year(meta: &CinemetaMeta) -> Option<u16> {
    if let Some(val) = &meta.year {
        if let Some(num) = val.as_u64() {
            return u16::try_from(num).ok();
        }
        if let Some(text) = val.as_str() {
            return parse_year_prefix(Some(text));
        }
    }
    parse_year_prefix(meta.release_info.as_deref())
}

fn cinemeta_to_canonical(meta: CinemetaMeta) -> CanonicalMetadata {
    let year = parse_cinemeta_year(&meta);
    let mut season_counts: BTreeMap<u32, u32> = BTreeMap::new();
    for video in &meta.videos {
        if let (Some(season), Some(_ep)) = (video.season, video.episode)
            && season > 0
        {
            *season_counts.entry(season).or_insert(0) += 1;
        }
    }
    let season_spans: Vec<SeasonSpan> = season_counts
        .into_iter()
        .map(|(season, episode_count)| SeasonSpan {
            season,
            episode_count,
        })
        .collect();
    let episodes: Vec<CanonicalEpisodeMeta> = meta
        .videos
        .into_iter()
        .filter_map(|video| {
            let season = video.season?;
            let episode = video.episode?;
            if season == 0 || episode == 0 {
                return None;
            }
            let title = video
                .title
                .or(video.name)
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty());
            let overview = video
                .overview
                .or(video.description)
                .map(|o| o.trim().to_owned())
                .filter(|o| !o.is_empty());
            let thumbnail = video
                .thumbnail
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty());
            let air_date = video
                .released
                .map(|d| d.trim().to_owned())
                .filter(|d| !d.is_empty());
            Some(CanonicalEpisodeMeta {
                season,
                episode,
                absolute_episode: season_episode_to_absolute(&season_spans, season, episode),
                title,
                overview,
                thumbnail,
                air_date,
            })
        })
        .collect();
    let imdb = meta.id.starts_with("tt").then(|| meta.id.clone());
    let rating = meta
        .imdb_rating
        .as_deref()
        .and_then(|s| s.parse::<f32>().ok());
    let trailer_url = meta
        .trailers
        .iter()
        .filter(|t| {
            t.kind
                .as_deref()
                .is_none_or(|k| k.eq_ignore_ascii_case("trailer"))
        })
        .find_map(|t| t.source.as_deref().and_then(youtube_watch_url))
        .or_else(|| {
            meta.trailers
                .iter()
                .find_map(|t| t.source.as_deref().and_then(youtube_watch_url))
        })
        .or_else(|| {
            meta.trailer_streams
                .iter()
                .find_map(|s| s.yt_id.as_deref().and_then(youtube_watch_url))
        });

    CanonicalMetadata {
        canonical_id: format!("imdb:{}", meta.id),
        external_ids: ExternalIds {
            imdb,
            ..ExternalIds::default()
        },
        title: meta.name.unwrap_or_else(|| meta.id.clone()),
        original_title: None,
        year,
        synopsis: meta.description,
        synopsis_lang: Some("en".into()),
        poster: meta.poster,
        backdrop: meta.background,
        trailer_url,
        genres: meta.genres,
        rating,
        season_spans,
        episodes,
    }
}

/// HTTP metadata lookup client supporting AniList, TMDB, and keyless Cinemeta fallback.
#[derive(Debug, Clone)]
pub struct MetadataClients {
    http: reqwest::Client,
    endpoints: MetaEndpoints,
}

impl Default for MetadataClients {
    fn default() -> Self {
        Self::new(MetaEndpoints::default())
    }
}

impl MetadataClients {
    /// Creates a metadata client pointing at `endpoints`.
    pub fn new(endpoints: MetaEndpoints) -> Self {
        let http = vessel_domain::http_client_builder()
            .timeout(METADATA_TIMEOUT)
            .build()
            .expect("build metadata reqwest client");
        Self { http, endpoints }
    }

    /// Resolves canonical metadata via exact IDs first, then safe confidence-scored search.
    pub async fn resolve_for_detail(
        &self,
        detail: &MediaDetail,
        preferred_lang: &str,
        tmdb_api_key: Option<&str>,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        self.resolve_for_detail_with_keys(detail, preferred_lang, tmdb_api_key, None)
            .await
    }

    /// Resolves canonical metadata with optional TMDB API key and optional Simkl Client ID.
    pub async fn resolve_for_detail_with_keys(
        &self,
        detail: &MediaDetail,
        preferred_lang: &str,
        tmdb_api_key: Option<&str>,
        simkl_client_id: Option<&str>,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        if let Some(anilist_id) = detail.external_ids.anilist
            && let Some(mut meta) = self.fetch_anilist_by_id(anilist_id).await?
        {
            self.enrich_anime_episodes_by(
                "anilist_id",
                anilist_id,
                preferred_lang,
                tmdb_api_key,
                &mut meta,
            )
            .await;
            self.fill_missing_trailer(detail.media_type, preferred_lang, tmdb_api_key, &mut meta)
                .await;
            return Ok(Some(meta));
        }

        if let Some(mal_id) = detail.external_ids.mal
            && let Some(mut meta) = self.fetch_mal_by_id(mal_id).await?
        {
            self.enrich_anime_episodes_by(
                "mal_id",
                mal_id,
                preferred_lang,
                tmdb_api_key,
                &mut meta,
            )
            .await;
            self.fill_missing_trailer(detail.media_type, preferred_lang, tmdb_api_key, &mut meta)
                .await;
            return Ok(Some(meta));
        }

        if let Some(kitsu_id) = detail.external_ids.kitsu
            && let Some(mut meta) = self.fetch_kitsu_by_id(kitsu_id).await?
        {
            self.enrich_anime_episodes_by(
                "kitsu_id",
                kitsu_id,
                preferred_lang,
                tmdb_api_key,
                &mut meta,
            )
            .await;
            self.fill_missing_trailer(detail.media_type, preferred_lang, tmdb_api_key, &mut meta)
                .await;
            return Ok(Some(meta));
        }

        if let Some(simkl_id) = detail.external_ids.simkl
            && let Some(client_id) = simkl_client_id.filter(|k| !k.trim().is_empty())
            && let Some(mut meta) = self
                .fetch_simkl_by_id(simkl_id, detail.media_type, client_id)
                .await?
        {
            self.fill_missing_trailer(detail.media_type, preferred_lang, tmdb_api_key, &mut meta)
                .await;
            return Ok(Some(meta));
        }

        if let Some(tmdb_id) = detail.external_ids.tmdb
            && let Some(key) = tmdb_api_key.filter(|k| !k.trim().is_empty())
            && let Some(mut meta) = self
                .fetch_tmdb_by_id(tmdb_id, detail.media_type, preferred_lang, key)
                .await?
        {
            self.fill_missing_trailer(detail.media_type, preferred_lang, tmdb_api_key, &mut meta)
                .await;
            return Ok(Some(meta));
        }

        if let Some(ref imdb_id) = detail.external_ids.imdb
            && let Some(meta) = self
                .fetch_cinemeta_any_type(imdb_id, detail.media_type)
                .await?
        {
            return Ok(Some(meta));
        }

        if detail.is_anime
            && let Some(mut meta) = self
                .search_anilist_confident(&detail.title, detail.year)
                .await?
        {
            if let Some(anilist_id) = meta.external_ids.anilist {
                self.enrich_anime_episodes_by(
                    "anilist_id",
                    anilist_id,
                    preferred_lang,
                    tmdb_api_key,
                    &mut meta,
                )
                .await;
            }
            self.fill_missing_trailer(detail.media_type, preferred_lang, tmdb_api_key, &mut meta)
                .await;
            return Ok(Some(meta));
        }

        if let Some(key) = tmdb_api_key.filter(|k| !k.trim().is_empty())
            && let Some(mut meta) = self
                .search_tmdb_confident(
                    &detail.title,
                    detail.year,
                    detail.media_type,
                    preferred_lang,
                    key,
                )
                .await?
        {
            self.fill_missing_trailer(detail.media_type, preferred_lang, tmdb_api_key, &mut meta)
                .await;
            return Ok(Some(meta));
        }

        self.search_cinemeta_confident(&detail.title, detail.year, detail.media_type)
            .await
    }

    async fn fill_missing_trailer(
        &self,
        media_type: MediaType,
        preferred_lang: &str,
        tmdb_api_key: Option<&str>,
        meta: &mut CanonicalMetadata,
    ) {
        if meta.trailer_url.is_some() {
            return;
        }
        if let (Some(tmdb_id), Some(key)) = (
            meta.external_ids.tmdb,
            tmdb_api_key.filter(|k| !k.trim().is_empty()),
        ) && let Ok(Some(tmdb_meta)) = self
            .fetch_tmdb_by_id(tmdb_id, media_type, preferred_lang, key)
            .await
            && tmdb_meta.trailer_url.is_some()
        {
            meta.trailer_url = tmdb_meta.trailer_url;
            return;
        }
        if let Some(ref imdb_id) = meta.external_ids.imdb
            && let Ok(Some(cm_meta)) = self.fetch_cinemeta_any_type(imdb_id, media_type).await
            && cm_meta.trailer_url.is_some()
        {
            meta.trailer_url = cm_meta.trailer_url;
        }
    }

    async fn fetch_anilist_by_id(&self, id: u64) -> VesselResult<Option<CanonicalMetadata>> {
        let candidates = self
            .execute_anilist_candidates(AniListRequest {
                query: ANILIST_BY_ID_QUERY,
                variables: serde_json::json!({ "id": id }),
            })
            .await?;
        Ok(candidates.into_iter().next().map(anilist_to_canonical))
    }

    async fn enrich_anime_episodes_by(
        &self,
        id_param: &str,
        id_value: u64,
        preferred_lang: &str,
        tmdb_api_key: Option<&str>,
        meta: &mut CanonicalMetadata,
    ) {
        // Keep test harnesses deterministic & offline when endpoints target localhost mocks.
        if self.endpoints.anilist_graphql.contains("127.0.0.1")
            || self.endpoints.anilist_graphql.contains("localhost")
        {
            return;
        }

        let url = format!("https://api.ani.zip/mappings?{id_param}={id_value}");
        if let Ok(resp) = self.http.get(&url).send().await
            && resp.status().is_success()
            && let Ok(body) = resp.json::<AniZipResponse>().await
        {
            if let Some(maps) = body.mappings {
                meta.external_ids.imdb = meta.external_ids.imdb.clone().or(maps.imdb_id);
                meta.external_ids.tmdb = meta.external_ids.tmdb.or(maps.themoviedb_id);
                meta.external_ids.mal = meta.external_ids.mal.or(maps.mal_id);
            }

            let mut season_counts: BTreeMap<u32, u32> = BTreeMap::new();
            let mut eps = Vec::new();
            for (key, ep) in body.episodes {
                let ep_num = ep
                    .episode_number
                    .or_else(|| key.parse::<u32>().ok())
                    .unwrap_or(0);
                let season_num = ep.season_number.unwrap_or(1);
                if ep_num == 0 || season_num == 0 {
                    continue;
                }
                *season_counts.entry(season_num).or_insert(0) += 1;
                let title = ep.title.and_then(|map| {
                    let pick_non_generic = |lang: &str| {
                        map.get(lang)
                            .map(|s| s.trim().to_owned())
                            .filter(|s| !crate::coordinates::is_generic_episode_title(s))
                    };
                    pick_non_generic(preferred_lang)
                        .or_else(|| pick_non_generic("en"))
                        .or_else(|| pick_non_generic("x-jat"))
                        .or_else(|| pick_non_generic("ja"))
                        .or_else(|| {
                            map.values()
                                .map(|s| s.trim().to_owned())
                                .find(|s| !crate::coordinates::is_generic_episode_title(s))
                        })
                });
                eps.push(CanonicalEpisodeMeta {
                    season: season_num,
                    episode: ep_num,
                    absolute_episode: ep.absolute_episode_number,
                    title,
                    overview: ep
                        .overview
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty()),
                    thumbnail: ep
                        .image
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty()),
                    air_date: ep
                        .air_date
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty()),
                });
            }
            if !eps.is_empty() {
                eps.sort_by_key(|e| (e.season, e.episode, e.absolute_episode));
                meta.episodes = eps;
                if season_counts.len() > 1 || !season_counts.is_empty() {
                    meta.season_spans = season_counts
                        .into_iter()
                        .map(|(season, episode_count)| SeasonSpan {
                            season,
                            episode_count,
                        })
                        .collect();
                }
                return;
            }
        }

        // Secondary fallback if AniZip had no episodes: check TMDB (if key present) or Cinemeta by IMDb ID.
        if let (Some(tmdb_id), Some(key)) = (
            meta.external_ids.tmdb,
            tmdb_api_key.filter(|k| !k.trim().is_empty()),
        ) && let Ok(Some(tmdb_meta)) = self
            .fetch_tmdb_by_id(tmdb_id, MediaType::Series, preferred_lang, key)
            .await
            && !tmdb_meta.episodes.is_empty()
        {
            meta.episodes = tmdb_meta.episodes;
            if !tmdb_meta.season_spans.is_empty() {
                meta.season_spans = tmdb_meta.season_spans;
            }
            return;
        }
        if let Some(ref imdb_id) = meta.external_ids.imdb
            && let Ok(Some(cm_meta)) = self.fetch_cinemeta_by_id(imdb_id, MediaType::Series).await
            && !cm_meta.episodes.is_empty()
        {
            meta.episodes = cm_meta.episodes;
            if !cm_meta.season_spans.is_empty() {
                meta.season_spans = cm_meta.season_spans;
            }
        }
    }

    async fn search_anilist_confident(
        &self,
        title: &str,
        year: Option<u16>,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        let clean = clean_search_title(title);
        let candidates = self
            .execute_anilist_candidates(AniListRequest {
                query: ANILIST_SEARCH_QUERY,
                variables: serde_json::json!({ "search": clean }),
            })
            .await?;

        let best = candidates
            .into_iter()
            .filter_map(|media| {
                let mut cand_titles: Vec<&str> = Vec::new();
                if let Some(ref t) = media.title {
                    if let Some(ref e) = t.english {
                        cand_titles.push(e.as_str());
                    }
                    if let Some(ref r) = t.romaji {
                        cand_titles.push(r.as_str());
                    }
                    if let Some(ref n) = t.native {
                        cand_titles.push(n.as_str());
                    }
                }
                let confidence =
                    title_match_confidence(title, year, &cand_titles, media.season_year);
                (confidence >= MIN_MATCH_CONFIDENCE)
                    .then(|| (confidence, anilist_to_canonical(media)))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));

        Ok(best.map(|(_, meta)| meta))
    }

    async fn execute_anilist_candidates(
        &self,
        body: AniListRequest,
    ) -> VesselResult<Vec<AniListMedia>> {
        let resp = self
            .http
            .post(&self.endpoints.anilist_graphql)
            .json(&body)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("AniList request failed: {err}")))?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let parsed: AniListResponse = resp
            .json()
            .await
            .map_err(|err| VesselError::Network(format!("decode AniList response: {err}")))?;
        let Some(data) = parsed.data else {
            return Ok(Vec::new());
        };
        if let Some(page) = data.page
            && !page.media.is_empty()
        {
            return Ok(page.media);
        }
        Ok(data.media.into_iter().collect())
    }

    async fn fetch_tmdb_by_id(
        &self,
        id: u64,
        media_type: MediaType,
        preferred_lang: &str,
        api_key: &str,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        let kind = match media_type {
            MediaType::Movie => "movie",
            MediaType::Series | MediaType::Live => "tv",
        };
        let lang = map_tmdb_lang(preferred_lang);
        let base_lang = lang.split('-').next().unwrap_or("en");
        let url = format!(
            "{}/{kind}/{id}?api_key={api_key}&language={lang}&append_to_response=external_ids,videos&include_video_language={base_lang},en,null",
            self.endpoints.tmdb_api_base
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("TMDB detail failed: {err}")))?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let detail: TmdbDetail = resp
            .json()
            .await
            .map_err(|err| VesselError::Network(format!("decode TMDB detail: {err}")))?;

        let poster = detail
            .poster_path
            .as_ref()
            .map(|p| format!("{}/w780{p}", self.endpoints.tmdb_image_base));
        let backdrop = detail
            .backdrop_path
            .as_ref()
            .map(|p| format!("{}/w1280{p}", self.endpoints.tmdb_image_base));
        let external_ids = ExternalIds {
            tmdb: Some(detail.id),
            imdb: detail.external_ids.as_ref().and_then(|e| e.imdb_id.clone()),
            tvdb: detail.external_ids.and_then(|e| e.tvdb_id),
            ..ExternalIds::default()
        };
        let canonical_id = canonical_id_from_external_ids(&external_ids, media_type)
            .unwrap_or_else(|| format!("tmdb:{kind}:{}", detail.id));
        let year = parse_year_prefix(
            detail
                .release_date
                .as_deref()
                .or(detail.first_air_date.as_deref()),
        );
        let season_spans: Vec<SeasonSpan> = detail
            .seasons
            .into_iter()
            .filter(|s| s.season_number > 0 && s.episode_count > 0)
            .map(|s| SeasonSpan {
                season: s.season_number,
                episode_count: s.episode_count,
            })
            .collect();

        let mut episodes: Vec<CanonicalEpisodeMeta> = Vec::new();
        if kind == "tv" && !season_spans.is_empty() {
            let mut set = tokio::task::JoinSet::new();
            for span in season_spans.iter().take(5) {
                let s_num = span.season;
                let s_url = format!(
                    "{}/tv/{}/season/{}?api_key={}&language={}",
                    self.endpoints.tmdb_api_base, detail.id, s_num, api_key, lang
                );
                let http = self.http.clone();
                let img_base = self.endpoints.tmdb_image_base.clone();
                set.spawn(async move {
                    let resp = http.get(&s_url).send().await.ok()?;
                    if !resp.status().is_success() {
                        return None;
                    }
                    let body: TmdbSeasonDetail = resp.json().await.ok()?;
                    Some((s_num, img_base, body.episodes))
                });
            }
            while let Some(res) = set.join_next().await {
                if let Ok(Some((s_num, img_base, eps))) = res {
                    for ep in eps {
                        if ep.episode_number == 0 {
                            continue;
                        }
                        episodes.push(CanonicalEpisodeMeta {
                            season: s_num,
                            episode: ep.episode_number,
                            absolute_episode: season_episode_to_absolute(
                                &season_spans,
                                s_num,
                                ep.episode_number,
                            ),
                            title: ep.name.filter(|s| !s.trim().is_empty()),
                            overview: ep.overview.filter(|s| !s.trim().is_empty()),
                            thumbnail: ep.still_path.map(|p| format!("{img_base}/w500{p}")),
                            air_date: ep.air_date.filter(|s| !s.trim().is_empty()),
                        });
                    }
                }
            }
            episodes.sort_by_key(|e| (e.season, e.episode));
        }

        let synopsis_lang = detail
            .overview
            .as_deref()
            .filter(|o| !o.trim().is_empty())
            .map(|_| preferred_lang.to_owned());
        let trailer_url = detail.videos.and_then(|vlist| {
            let mut ranked: Vec<(i32, String)> = vlist
                .results
                .into_iter()
                .filter_map(|v| {
                    if !v
                        .site
                        .as_deref()
                        .is_some_and(|s| s.eq_ignore_ascii_case("youtube"))
                    {
                        return None;
                    }
                    let url = v.key.as_deref().and_then(youtube_watch_url)?;
                    let mut score = 0;
                    if v.kind
                        .as_deref()
                        .is_some_and(|k| k.eq_ignore_ascii_case("trailer"))
                    {
                        score += 100;
                    } else if v
                        .kind
                        .as_deref()
                        .is_some_and(|k| k.eq_ignore_ascii_case("teaser"))
                    {
                        score += 50;
                    }
                    if v.official.unwrap_or(false) {
                        score += 20;
                    }
                    if v.iso_639_1
                        .as_deref()
                        .is_some_and(|l| l.eq_ignore_ascii_case(base_lang))
                    {
                        score += 10;
                    } else if v
                        .iso_639_1
                        .as_deref()
                        .is_some_and(|l| l.eq_ignore_ascii_case("en"))
                    {
                        score += 5;
                    }
                    Some((score, url))
                })
                .collect();
            ranked.sort_by(|a, b| b.0.cmp(&a.0));
            ranked.into_iter().next().map(|(_, u)| u)
        });

        Ok(Some(CanonicalMetadata {
            canonical_id,
            external_ids,
            title: detail
                .title
                .or(detail.name)
                .unwrap_or_else(|| format!("TMDB #{}", detail.id)),
            original_title: detail.original_title.or(detail.original_name),
            year,
            synopsis: detail.overview.filter(|o| !o.trim().is_empty()),
            synopsis_lang,
            poster,
            backdrop,
            trailer_url,
            genres: detail.genres.into_iter().map(|g| g.name).collect(),
            rating: detail.vote_average,
            season_spans,
            episodes,
        }))
    }

    async fn search_tmdb_confident(
        &self,
        title: &str,
        year: Option<u16>,
        media_type: MediaType,
        preferred_lang: &str,
        api_key: &str,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        let kind = match media_type {
            MediaType::Movie => "movie",
            MediaType::Series | MediaType::Live => "tv",
        };
        let clean = clean_search_title(title);
        let lang = map_tmdb_lang(preferred_lang);
        let resp = self
            .http
            .get(format!("{}/search/{kind}", self.endpoints.tmdb_api_base))
            .query(&[
                ("api_key", api_key),
                ("query", clean.as_str()),
                ("language", &lang),
            ])
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("TMDB search failed: {err}")))?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let body: TmdbSearchResponse = resp
            .json()
            .await
            .map_err(|err| VesselError::Network(format!("decode TMDB search: {err}")))?;

        let best = body
            .results
            .into_iter()
            .filter_map(|item| {
                let t1 = item.title.as_deref().or(item.name.as_deref()).unwrap_or("");
                let t2 = item
                    .original_title
                    .as_deref()
                    .or(item.original_name.as_deref())
                    .unwrap_or("");
                let cand_year = parse_year_prefix(
                    item.release_date
                        .as_deref()
                        .or(item.first_air_date.as_deref()),
                );
                let score = title_match_confidence(title, year, &[t1, t2], cand_year);
                (score >= MIN_MATCH_CONFIDENCE).then_some((score, item.id))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));

        match best {
            Some((_, id)) => {
                self.fetch_tmdb_by_id(id, media_type, preferred_lang, api_key)
                    .await
            }
            None => Ok(None),
        }
    }

    async fn fetch_cinemeta_by_id(
        &self,
        imdb_id: &str,
        media_type: MediaType,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        let kind = match media_type {
            MediaType::Movie => "movie",
            MediaType::Series | MediaType::Live => "series",
        };
        let url = format!(
            "{}/meta/{kind}/{imdb_id}.json",
            self.endpoints.cinemeta_base
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("Cinemeta detail failed: {err}")))?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let parsed: CinemetaMetaEnvelope = resp
            .json()
            .await
            .map_err(|err| VesselError::Network(format!("decode Cinemeta detail: {err}")))?;
        Ok(parsed.meta.map(cinemeta_to_canonical))
    }

    async fn search_cinemeta_confident(
        &self,
        title: &str,
        year: Option<u16>,
        media_type: MediaType,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        let kind = match media_type {
            MediaType::Movie => "movie",
            MediaType::Series | MediaType::Live => "series",
        };
        let clean = clean_search_title(title);
        let url = format!(
            "{}/catalog/{kind}/top/search={}.json",
            self.endpoints.cinemeta_base,
            encode_path_segment(&clean)
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("Cinemeta search failed: {err}")))?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let parsed: CinemetaCatalogEnvelope = resp
            .json()
            .await
            .map_err(|err| VesselError::Network(format!("decode Cinemeta catalog: {err}")))?;

        let best = parsed
            .metas
            .into_iter()
            .filter_map(|meta| {
                let name = meta.name.as_deref().unwrap_or("");
                let cand_year = parse_cinemeta_year(&meta);
                let score = title_match_confidence(title, year, &[name], cand_year);
                (score >= MIN_MATCH_CONFIDENCE).then_some((score, meta))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));

        match best {
            Some((_, hit)) => match self.fetch_cinemeta_by_id(&hit.id, media_type).await? {
                Some(full) => Ok(Some(full)),
                None => Ok(Some(cinemeta_to_canonical(hit))),
            },
            None => Ok(None),
        }
    }

    async fn fetch_cinemeta_any_type(
        &self,
        imdb_id: &str,
        preferred: MediaType,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        if let Some(hit) = self.fetch_cinemeta_by_id(imdb_id, preferred).await? {
            return Ok(Some(hit));
        }
        let alt = match preferred {
            MediaType::Movie => MediaType::Series,
            MediaType::Series | MediaType::Live => MediaType::Movie,
        };
        self.fetch_cinemeta_by_id(imdb_id, alt).await
    }

    async fn fetch_mal_by_id(&self, mal_id: u64) -> VesselResult<Option<CanonicalMetadata>> {
        let url = format!("{}/anime/{mal_id}/full", self.endpoints.jikan_base);
        let resp = self.http.get(&url).send().await.map_err(|err| {
            VesselError::Network(format!("[VesselNetworkError]: Jikan detail failed: {err}"))
        })?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let parsed: JikanSingleResponse = resp.json().await.map_err(|err| {
            VesselError::Network(format!("[VesselNetworkError]: decode Jikan detail: {err}"))
        })?;
        Ok(parsed.data.map(jikan_to_canonical))
    }

    async fn fetch_kitsu_by_id(&self, kitsu_id: u64) -> VesselResult<Option<CanonicalMetadata>> {
        let url = format!("{}/anime/{kitsu_id}", self.endpoints.kitsu_base);
        let resp = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.api+json")
            .send()
            .await
            .map_err(|err| {
                VesselError::Network(format!("[VesselNetworkError]: Kitsu detail failed: {err}"))
            })?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let parsed: KitsuSingleResponse = resp.json().await.map_err(|err| {
            VesselError::Network(format!("[VesselNetworkError]: decode Kitsu detail: {err}"))
        })?;
        Ok(parsed.data.and_then(kitsu_to_canonical))
    }

    async fn fetch_simkl_by_id(
        &self,
        simkl_id: u64,
        preferred: MediaType,
        client_id: &str,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        let paths = match preferred {
            MediaType::Movie => ["movies", "anime", "tv"],
            MediaType::Series | MediaType::Live => ["tv", "anime", "movies"],
        };
        for kind_path in paths {
            let url = format!(
                "{}/{kind_path}/{simkl_id}?extended=full&client_id={client_id}",
                self.endpoints.simkl_base
            );
            let Ok(resp) = self.http.get(&url).send().await else {
                continue;
            };
            if !resp.status().is_success() {
                continue;
            }
            if let Ok(detail) = resp.json::<SimklDetail>().await {
                return Ok(Some(simkl_to_canonical(simkl_id, detail)));
            }
        }
        Ok(None)
    }

    /// Interactive title search across TMDB, IMDb, Simkl, AniList, MyAnimeList, and Kitsu for Fix Match.
    pub async fn search_interactive(
        &self,
        query: &str,
        provider: MetadataProviderFilter,
        preferred_lang: &str,
        tmdb_api_key: Option<&str>,
        simkl_client_id: Option<&str>,
    ) -> MetadataSearchResponse {
        let clean = clean_search_title(query);
        let q = if clean.is_empty() {
            query.trim().to_owned()
        } else {
            clean
        };
        if q.is_empty() {
            return MetadataSearchResponse::default();
        }

        let tmdb_key = tmdb_api_key
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(str::to_owned);
        let simkl_key = simkl_client_id
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(str::to_owned);

        let want =
            |p: MetadataProviderFilter| provider == MetadataProviderFilter::All || provider == p;
        let mut notes = Vec::new();

        if want(MetadataProviderFilter::Tmdb)
            && tmdb_key.is_none()
            && provider == MetadataProviderFilter::Tmdb
        {
            notes.push("TMDB search needs an optional TMDB API key in Settings (keyless IMDb, AniList, MyAnimeList & Kitsu work without a key).".into());
        }
        if want(MetadataProviderFilter::Simkl)
            && simkl_key.is_none()
            && provider == MetadataProviderFilter::Simkl
        {
            notes.push("Simkl search requires a Simkl Client ID in Settings (or enter a manual Simkl ID below).".into());
        }

        let anilist_fut = async {
            if want(MetadataProviderFilter::Anilist) {
                self.search_anilist_hits(&q).await
            } else {
                Vec::new()
            }
        };
        let mal_fut = async {
            if want(MetadataProviderFilter::Mal) {
                self.search_mal_hits(&q).await
            } else {
                Vec::new()
            }
        };
        let kitsu_fut = async {
            if want(MetadataProviderFilter::Kitsu) {
                self.search_kitsu_hits(&q).await
            } else {
                Vec::new()
            }
        };
        let imdb_fut = async {
            if want(MetadataProviderFilter::Imdb) {
                self.search_imdb_hits(&q).await
            } else {
                Vec::new()
            }
        };
        let tmdb_fut = async {
            if want(MetadataProviderFilter::Tmdb) {
                if let Some(ref key) = tmdb_key {
                    self.search_tmdb_hits(&q, preferred_lang, key).await
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        };
        let simkl_fut = async {
            if want(MetadataProviderFilter::Simkl) {
                if let Some(ref cid) = simkl_key {
                    self.search_simkl_hits(&q, cid).await
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        };

        let (tmdb_hits, imdb_hits, simkl_hits, anilist_hits, mal_hits, kitsu_hits) = tokio::join!(
            tmdb_fut,
            imdb_fut,
            simkl_fut,
            anilist_fut,
            mal_fut,
            kitsu_fut
        );

        let mut hits = Vec::new();
        hits.extend(tmdb_hits);
        hits.extend(imdb_hits);
        hits.extend(simkl_hits);
        hits.extend(anilist_hits);
        hits.extend(mal_hits);
        hits.extend(kitsu_hits);

        MetadataSearchResponse { hits, notes }
    }

    async fn search_anilist_hits(&self, query: &str) -> Vec<MetadataSearchHit> {
        let Ok(items) = self
            .execute_anilist_candidates(AniListRequest {
                query: ANILIST_SEARCH_QUERY,
                variables: serde_json::json!({ "search": query }),
            })
            .await
        else {
            return Vec::new();
        };
        items
            .into_iter()
            .map(|m| {
                let c = anilist_to_canonical(m);
                MetadataSearchHit {
                    canonical_id: c.canonical_id,
                    provider: "anilist".into(),
                    title: c.title,
                    original_title: c.original_title,
                    year: c.year,
                    media_type: Some("anime".into()),
                    poster: c.poster,
                    synopsis: c.synopsis.map(|s| s.chars().take(220).collect()),
                    external_ids: c.external_ids,
                }
            })
            .collect()
    }

    async fn search_mal_hits(&self, query: &str) -> Vec<MetadataSearchHit> {
        let Ok(resp) = self
            .http
            .get(format!("{}/anime", self.endpoints.jikan_base))
            .query(&[("q", query), ("limit", "8")])
            .send()
            .await
        else {
            return Vec::new();
        };
        if !resp.status().is_success() {
            return Vec::new();
        }
        let Ok(parsed) = resp.json::<JikanListResponse>().await else {
            return Vec::new();
        };
        parsed
            .data
            .into_iter()
            .map(|item| {
                let c = jikan_to_canonical(item);
                MetadataSearchHit {
                    canonical_id: c.canonical_id,
                    provider: "mal".into(),
                    title: c.title,
                    original_title: c.original_title,
                    year: c.year,
                    media_type: Some("anime".into()),
                    poster: c.poster,
                    synopsis: c.synopsis.map(|s| s.chars().take(220).collect()),
                    external_ids: c.external_ids,
                }
            })
            .collect()
    }

    async fn search_kitsu_hits(&self, query: &str) -> Vec<MetadataSearchHit> {
        let Ok(resp) = self
            .http
            .get(format!("{}/anime", self.endpoints.kitsu_base))
            .header("Accept", "application/vnd.api+json")
            .query(&[("filter[text]", query), ("page[limit]", "8")])
            .send()
            .await
        else {
            return Vec::new();
        };
        if !resp.status().is_success() {
            return Vec::new();
        }
        let Ok(parsed) = resp.json::<KitsuListResponse>().await else {
            return Vec::new();
        };
        parsed
            .data
            .into_iter()
            .filter_map(kitsu_to_canonical)
            .map(|c| MetadataSearchHit {
                canonical_id: c.canonical_id,
                provider: "kitsu".into(),
                title: c.title,
                original_title: c.original_title,
                year: c.year,
                media_type: Some("anime".into()),
                poster: c.poster,
                synopsis: c.synopsis.map(|s| s.chars().take(220).collect()),
                external_ids: c.external_ids,
            })
            .collect()
    }

    async fn search_imdb_hits(&self, query: &str) -> Vec<MetadataSearchHit> {
        let first_ch = query
            .chars()
            .find(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .unwrap_or('a');
        let suggest_url = format!(
            "{}/{}/{}.json",
            self.endpoints.imdb_suggest_base,
            first_ch,
            encode_path_segment(&query.to_ascii_lowercase())
        );

        let mut out = Vec::new();
        if let Ok(resp) = self.http.get(&suggest_url).send().await
            && resp.status().is_success()
            && let Ok(parsed) = resp.json::<ImdbSuggestEnvelope>().await
        {
            for item in parsed.d {
                if !item.id.starts_with("tt") {
                    continue;
                }
                let Some(title) = item.l.filter(|s| !s.trim().is_empty()) else {
                    continue;
                };
                let qid = item
                    .qid
                    .as_deref()
                    .or(item.q.as_deref())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let media_type = if qid.contains("series") || qid.contains("tv") {
                    "series"
                } else {
                    "movie"
                };
                out.push(MetadataSearchHit {
                    canonical_id: format!("imdb:{}", item.id),
                    provider: "imdb".into(),
                    title,
                    original_title: None,
                    year: item.y,
                    media_type: Some(media_type.into()),
                    poster: item.i.and_then(|img| img.image_url),
                    synopsis: None,
                    external_ids: ExternalIds {
                        imdb: Some(item.id),
                        ..ExternalIds::default()
                    },
                });
                if out.len() >= 8 {
                    return out;
                }
            }
        }

        if out.is_empty() {
            for kind in ["series", "movie"] {
                let url = format!(
                    "{}/catalog/{kind}/top/search={}.json",
                    self.endpoints.cinemeta_base,
                    encode_path_segment(query)
                );
                if let Ok(resp) = self.http.get(&url).send().await
                    && resp.status().is_success()
                    && let Ok(parsed) = resp.json::<CinemetaCatalogEnvelope>().await
                {
                    for meta in parsed.metas.into_iter().take(5) {
                        if !meta.id.starts_with("tt") {
                            continue;
                        }
                        let year = parse_cinemeta_year(&meta);
                        let title = meta.name.unwrap_or_else(|| meta.id.clone());
                        out.push(MetadataSearchHit {
                            canonical_id: format!("imdb:{}", meta.id),
                            provider: "imdb".into(),
                            title,
                            original_title: None,
                            year,
                            media_type: Some(kind.into()),
                            poster: meta.poster,
                            synopsis: meta.description.map(|s| s.chars().take(220).collect()),
                            external_ids: ExternalIds {
                                imdb: Some(meta.id),
                                ..ExternalIds::default()
                            },
                        });
                    }
                }
            }
        }
        out
    }

    async fn search_tmdb_hits(
        &self,
        query: &str,
        preferred_lang: &str,
        api_key: &str,
    ) -> Vec<MetadataSearchHit> {
        let lang = map_tmdb_lang(preferred_lang);
        let Ok(resp) = self
            .http
            .get(format!("{}/search/multi", self.endpoints.tmdb_api_base))
            .query(&[("api_key", api_key), ("query", query), ("language", &lang)])
            .send()
            .await
        else {
            return Vec::new();
        };
        if !resp.status().is_success() {
            return Vec::new();
        }
        let Ok(body) = resp.json::<TmdbSearchResponse>().await else {
            return Vec::new();
        };
        body.results
            .into_iter()
            .filter_map(|item| {
                let mtype = match item.media_type.as_deref() {
                    Some("movie") => "movie",
                    Some("tv") => "tv",
                    _ => return None,
                };
                let title = item.title.or(item.name)?;
                let original_title = item.original_title.or(item.original_name);
                let year = parse_year_prefix(
                    item.release_date
                        .as_deref()
                        .or(item.first_air_date.as_deref()),
                );
                let poster = item
                    .poster_path
                    .map(|p| format!("{}/w342{p}", self.endpoints.tmdb_image_base));
                Some(MetadataSearchHit {
                    canonical_id: format!("tmdb:{mtype}:{}", item.id),
                    provider: "tmdb".into(),
                    title,
                    original_title,
                    year,
                    media_type: Some(if mtype == "tv" {
                        "series".into()
                    } else {
                        "movie".into()
                    }),
                    poster,
                    synopsis: item
                        .overview
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| s.chars().take(220).collect()),
                    external_ids: ExternalIds {
                        tmdb: Some(item.id),
                        ..ExternalIds::default()
                    },
                })
            })
            .take(8)
            .collect()
    }

    async fn search_simkl_hits(&self, query: &str, client_id: &str) -> Vec<MetadataSearchHit> {
        let mut out = Vec::new();
        for kind in ["tv", "anime", "movie"] {
            let Ok(resp) = self
                .http
                .get(format!("{}/search/{kind}", self.endpoints.simkl_base))
                .query(&[("q", query), ("limit", "4"), ("client_id", client_id)])
                .send()
                .await
            else {
                continue;
            };
            if !resp.status().is_success() {
                continue;
            }
            let Ok(items) = resp.json::<Vec<SimklSearchItem>>().await else {
                continue;
            };
            for item in items {
                let Some(ids) = item.ids else { continue };
                let Some(simkl_id) = ids.simkl.or(ids.simkl_id) else {
                    continue;
                };
                let Some(title) = item.title.filter(|s| !s.trim().is_empty()) else {
                    continue;
                };
                let poster = item
                    .poster
                    .map(|p| format!("https://simkl.in/posters/{p}_m.jpg"));
                let mtype = item.kind.unwrap_or_else(|| {
                    if kind == "movie" {
                        "movie".into()
                    } else if kind == "anime" {
                        "anime".into()
                    } else {
                        "series".into()
                    }
                });
                out.push(MetadataSearchHit {
                    canonical_id: format!("simkl:{simkl_id}"),
                    provider: "simkl".into(),
                    title,
                    original_title: None,
                    year: item.year,
                    media_type: Some(mtype),
                    poster,
                    synopsis: None,
                    external_ids: simkl_ids_to_external(simkl_id, &ids),
                });
            }
        }
        out
    }
}

fn json_u64(val: Option<&serde_json::Value>) -> Option<u64> {
    let v = val?;
    v.as_u64()
        .or_else(|| v.as_str()?.trim().parse::<u64>().ok())
}

fn simkl_ids_to_external(simkl_id: u64, ids: &SimklIds) -> ExternalIds {
    ExternalIds {
        simkl: Some(simkl_id),
        imdb: ids.imdb.clone().filter(|s| !s.trim().is_empty()),
        tmdb: json_u64(ids.tmdb.as_ref()),
        mal: json_u64(ids.mal.as_ref()),
        anilist: json_u64(ids.anilist.as_ref()),
        kitsu: json_u64(ids.kitsu.as_ref()),
        ..ExternalIds::default()
    }
}

fn simkl_to_canonical(simkl_id: u64, d: SimklDetail) -> CanonicalMetadata {
    let external_ids = d
        .ids
        .as_ref()
        .map(|ids| simkl_ids_to_external(simkl_id, ids))
        .unwrap_or(ExternalIds {
            simkl: Some(simkl_id),
            ..ExternalIds::default()
        });
    let season_spans = d
        .total_episodes
        .filter(|c| *c > 0)
        .map(|c| {
            vec![SeasonSpan {
                season: 1,
                episode_count: c,
            }]
        })
        .unwrap_or_default();
    CanonicalMetadata {
        canonical_id: format!("simkl:{simkl_id}"),
        external_ids,
        title: d
            .en_title
            .or(d.title)
            .unwrap_or_else(|| format!("Simkl #{simkl_id}")),
        original_title: None,
        year: d.year,
        synopsis: d.overview.filter(|s| !s.trim().is_empty()),
        synopsis_lang: Some("en".into()),
        poster: d
            .poster
            .map(|p| format!("https://simkl.in/posters/{p}_m.jpg")),
        backdrop: d
            .fanart
            .map(|f| format!("https://simkl.in/fanart/{f}_medium.jpg")),
        trailer_url: None,
        genres: d.genres,
        rating: d.ratings.and_then(|r| r.simkl).and_then(|s| s.rating),
        season_spans,
        episodes: Vec::new(),
    }
}

fn jikan_to_canonical(a: JikanAnime) -> CanonicalMetadata {
    let year = a
        .year
        .or_else(|| parse_year_prefix(a.aired.as_ref().and_then(|x| x.from.as_deref())));
    let poster = a.images.and_then(|imgs| {
        imgs.jpg
            .and_then(|j| j.large_image_url.or(j.image_url))
            .or_else(|| imgs.webp.and_then(|w| w.large_image_url.or(w.image_url)))
    });
    let title = a
        .title_english
        .clone()
        .or_else(|| a.title.clone())
        .unwrap_or_else(|| format!("MAL #{}", a.mal_id));
    let season_spans = a
        .episodes
        .filter(|c| *c > 0)
        .map(|c| {
            vec![SeasonSpan {
                season: 1,
                episode_count: c,
            }]
        })
        .unwrap_or_default();
    let trailer_url = a.trailer.and_then(|t| {
        t.youtube_id
            .as_deref()
            .and_then(youtube_watch_url)
            .or_else(|| t.url.as_deref().and_then(youtube_watch_url))
    });
    CanonicalMetadata {
        canonical_id: format!("mal:{}", a.mal_id),
        external_ids: ExternalIds {
            mal: Some(a.mal_id),
            ..ExternalIds::default()
        },
        title,
        original_title: a.title_japanese.or(a.title),
        year,
        synopsis: a.synopsis.filter(|s| !s.trim().is_empty()),
        synopsis_lang: Some("en".into()),
        poster,
        backdrop: None,
        trailer_url,
        genres: a.genres.into_iter().map(|g| g.name).collect(),
        rating: a.score,
        season_spans,
        episodes: Vec::new(),
    }
}

fn kitsu_to_canonical(item: KitsuItem) -> Option<CanonicalMetadata> {
    let id = item.id.trim().parse::<u64>().ok()?;
    let attrs = item.attributes;
    let title = attrs
        .titles
        .get("en")
        .and_then(|o| o.clone())
        .or_else(|| attrs.titles.get("en_jp").and_then(|o| o.clone()))
        .or(attrs.canonical_title.clone())
        .unwrap_or_else(|| format!("Kitsu #{id}"));
    let original_title = attrs
        .titles
        .get("ja_jp")
        .and_then(|o| o.clone())
        .or(attrs.canonical_title);
    let poster = attrs
        .poster_image
        .and_then(|i| i.large.or(i.original).or(i.medium));
    let backdrop = attrs
        .cover_image
        .and_then(|i| i.large.or(i.original).or(i.medium));
    let rating = attrs
        .average_rating
        .as_deref()
        .and_then(|s| s.parse::<f32>().ok())
        .map(|v| v / 10.0);
    let season_spans = attrs
        .episode_count
        .filter(|c| *c > 0)
        .map(|c| {
            vec![SeasonSpan {
                season: 1,
                episode_count: c,
            }]
        })
        .unwrap_or_default();
    Some(CanonicalMetadata {
        canonical_id: format!("kitsu:{id}"),
        external_ids: ExternalIds {
            kitsu: Some(id),
            ..ExternalIds::default()
        },
        title,
        original_title,
        year: parse_year_prefix(attrs.start_date.as_deref()),
        synopsis: attrs.synopsis.filter(|s| !s.trim().is_empty()),
        synopsis_lang: Some("en".into()),
        poster,
        backdrop,
        trailer_url: attrs.youtube_video_id.as_deref().and_then(youtube_watch_url),
        genres: Vec::new(),
        rating,
        season_spans,
        episodes: Vec::new(),
    })
}
