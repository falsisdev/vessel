//! Metadata HTTP clients (AniList GraphQL, TMDB v3, keyless Cinemeta fallback) & fuzzy matching.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use vessel_domain::{ExternalIds, MediaDetail, MediaType, VesselError, VesselResult};

use crate::coordinates::SeasonSpan;
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
}

impl Default for MetaEndpoints {
    fn default() -> Self {
        Self {
            anilist_graphql: "https://graphql.anilist.co".into(),
            tmdb_api_base: "https://api.themoviedb.org/3".into(),
            tmdb_image_base: "https://image.tmdb.org/t/p".into(),
            cinemeta_base: "https://v3-cinemeta.strem.io".into(),
        }
    }
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
    let q_tokens = normalize_tokens(query_title);
    if q_tokens.is_empty() {
        return 0.0;
    }

    let mut best_text_score = 0.0_f32;
    for candidate in candidate_titles {
        let c_tokens = normalize_tokens(candidate);
        if c_tokens.is_empty() {
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
  }
}
"#;

const ANILIST_SEARCH_QUERY: &str = r#"
query ($search: String!) {
  Media(search: $search, type: ANIME, sort: [SEARCH_MATCH, POPULARITY_DESC]) {
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
    title: Option<String>,
    name: Option<String>,
    original_title: Option<String>,
    original_name: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
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
}

#[derive(Deserialize)]
struct CinemetaVideo {
    season: Option<u32>,
    episode: Option<u32>,
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
    let season_spans = season_counts
        .into_iter()
        .map(|(season, episode_count)| SeasonSpan {
            season,
            episode_count,
        })
        .collect();
    let imdb = meta.id.starts_with("tt").then(|| meta.id.clone());
    let rating = meta
        .imdb_rating
        .as_deref()
        .and_then(|s| s.parse::<f32>().ok());

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
        genres: meta.genres,
        rating,
        season_spans,
        episodes: Vec::new(),
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
        let http = reqwest::Client::builder()
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
        if let Some(anilist_id) = detail.external_ids.anilist
            && let Some(meta) = self.fetch_anilist_by_id(anilist_id).await?
        {
            return Ok(Some(meta));
        }

        if let Some(tmdb_id) = detail.external_ids.tmdb
            && let Some(key) = tmdb_api_key.filter(|k| !k.trim().is_empty())
            && let Some(meta) = self
                .fetch_tmdb_by_id(tmdb_id, detail.media_type, preferred_lang, key)
                .await?
        {
            return Ok(Some(meta));
        }

        if let Some(ref imdb_id) = detail.external_ids.imdb
            && let Some(meta) = self
                .fetch_cinemeta_by_id(imdb_id, detail.media_type)
                .await?
        {
            return Ok(Some(meta));
        }

        if detail.is_anime
            && let Some(meta) = self
                .search_anilist_confident(&detail.title, detail.year)
                .await?
        {
            return Ok(Some(meta));
        }

        if let Some(key) = tmdb_api_key.filter(|k| !k.trim().is_empty())
            && let Some(meta) = self
                .search_tmdb_confident(
                    &detail.title,
                    detail.year,
                    detail.media_type,
                    preferred_lang,
                    key,
                )
                .await?
        {
            return Ok(Some(meta));
        }

        self.search_cinemeta_confident(&detail.title, detail.year, detail.media_type)
            .await
    }

    async fn fetch_anilist_by_id(&self, id: u64) -> VesselResult<Option<CanonicalMetadata>> {
        self.execute_anilist(AniListRequest {
            query: ANILIST_BY_ID_QUERY,
            variables: serde_json::json!({ "id": id }),
        })
        .await
    }

    async fn search_anilist_confident(
        &self,
        title: &str,
        year: Option<u16>,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        let Some(meta) = self
            .execute_anilist(AniListRequest {
                query: ANILIST_SEARCH_QUERY,
                variables: serde_json::json!({ "search": title }),
            })
            .await?
        else {
            return Ok(None);
        };

        let mut cand_titles = vec![meta.title.as_str()];
        if let Some(ref orig) = meta.original_title {
            cand_titles.push(orig.as_str());
        }
        let confidence = title_match_confidence(title, year, &cand_titles, meta.year);
        if confidence >= MIN_MATCH_CONFIDENCE {
            Ok(Some(meta))
        } else {
            Ok(None)
        }
    }

    async fn execute_anilist(
        &self,
        body: AniListRequest,
    ) -> VesselResult<Option<CanonicalMetadata>> {
        let resp = self
            .http
            .post(&self.endpoints.anilist_graphql)
            .json(&body)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("AniList request failed: {err}")))?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let parsed: AniListResponse = resp
            .json()
            .await
            .map_err(|err| VesselError::Network(format!("decode AniList response: {err}")))?;
        Ok(parsed.data.and_then(|d| d.media).map(anilist_to_canonical))
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
        let url = format!(
            "{}/{kind}/{id}?api_key={api_key}&language={lang}&append_to_response=external_ids",
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
        let season_spans = detail
            .seasons
            .into_iter()
            .filter(|s| s.season_number > 0 && s.episode_count > 0)
            .map(|s| SeasonSpan {
                season: s.season_number,
                episode_count: s.episode_count,
            })
            .collect();

        let synopsis_lang = detail
            .overview
            .as_deref()
            .filter(|o| !o.trim().is_empty())
            .map(|_| preferred_lang.to_owned());

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
            genres: detail.genres.into_iter().map(|g| g.name).collect(),
            rating: detail.vote_average,
            season_spans,
            episodes: Vec::new(),
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
        let lang = map_tmdb_lang(preferred_lang);
        let resp = self
            .http
            .get(format!("{}/search/{kind}", self.endpoints.tmdb_api_base))
            .query(&[("api_key", api_key), ("query", title), ("language", &lang)])
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
        let url = format!(
            "{}/catalog/{kind}/top/search={}.json",
            self.endpoints.cinemeta_base,
            title.replace(' ', "%20")
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
}
