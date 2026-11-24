//! Metadata enrichment service (`AniList`, `TMDB`, `Cinemeta`), non-destructive merging,
//! and `anthology#10` bidirectional episode coordinate conversion.

pub mod clients;
pub mod coordinates;
pub mod merge;
pub mod skip;

use vessel_domain::{MediaDetail, SkipSegment, VesselResult};

pub use clients::{MIN_MATCH_CONFIDENCE, MetaEndpoints, MetadataClients, title_match_confidence};
pub use coordinates::{
    CanonicalEpisodeMeta, SeasonSpan, absolute_to_season_episode, align_episode_metadata,
    season_episode_to_absolute,
};
pub use merge::{
    CanonicalMetadata, canonical_id_from_external_ids, merge_detail, merge_detail_with_override,
};
pub use skip::{SkipEndpoints, SkipQuery, resolve_skip_segments};

/// Outcome of running metadata enrichment on a plugin's `MediaDetail`.
#[derive(Debug, Clone, PartialEq)]
pub struct EnrichmentOutcome {
    pub detail: MediaDetail,
    pub canonical: Option<CanonicalMetadata>,
}

/// Pure metadata enrichment facade (respects user offline/privacy opt-out toggle).
#[derive(Debug, Clone)]
pub struct MetadataService {
    clients: MetadataClients,
    skip_http: reqwest::Client,
    skip_endpoints: SkipEndpoints,
}

impl Default for MetadataService {
    fn default() -> Self {
        Self {
            clients: MetadataClients::default(),
            skip_http: vessel_domain::http_client_builder()
                .connect_timeout(std::time::Duration::from_secs(4))
                .build()
                .expect("build skip reqwest client"),
            skip_endpoints: SkipEndpoints::default(),
        }
    }
}

impl MetadataService {
    /// Creates a `MetadataService` backed by custom upstream endpoints (or defaults).
    pub fn new(endpoints: MetaEndpoints) -> Self {
        Self {
            clients: MetadataClients::new(endpoints),
            ..Self::default()
        }
    }

    /// Borrows the underlying HTTP metadata client.
    pub fn clients(&self) -> &MetadataClients {
        &self.clients
    }

    /// Resolves verified skip segments (`intro`, `recap`, `credits`, `preview`) from plugin + external DBs.
    pub async fn resolve_skip_segments(
        &self,
        query: &SkipQuery,
        allow_external: bool,
    ) -> Vec<SkipSegment> {
        resolve_skip_segments(&self.skip_http, &self.skip_endpoints, query, allow_external).await
    }

    /// Enriches `raw_detail` if `allow_external` is enabled and a confident match exists.
    pub async fn enrich_detail(
        &self,
        raw_detail: &MediaDetail,
        preferred_lang: &str,
        tmdb_api_key: Option<&str>,
        allow_external: bool,
    ) -> VesselResult<EnrichmentOutcome> {
        if !allow_external {
            return Ok(EnrichmentOutcome {
                detail: raw_detail.clone(),
                canonical: None,
            });
        }

        let Some(canonical) = self
            .clients
            .resolve_for_detail(raw_detail, preferred_lang, tmdb_api_key)
            .await?
        else {
            return Ok(EnrichmentOutcome {
                detail: raw_detail.clone(),
                canonical: None,
            });
        };

        let merged = merge_detail(raw_detail, &canonical, preferred_lang);
        Ok(EnrichmentOutcome {
            detail: merged,
            canonical: Some(canonical),
        })
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::extract::Query;
    use axum::routing::{get, post};
    use serde::Deserialize;
    use tokio::net::TcpListener;
    use vessel_domain::{
        ContentRating, Episode, EpisodeTag, ExternalIds, MediaDetail, MediaType, PluginId,
    };

    use super::{MetaEndpoints, MetadataService};

    #[derive(Deserialize)]
    struct TmdbQuery {
        language: Option<String>,
    }

    async fn spawn_mock_metadata_servers() -> MetaEndpoints {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let base = format!("http://{addr}");

        let app = Router::new()
            .route(
                "/anilist",
                post(|axum::Json(body): axum::Json<serde_json::Value>| async move {
                    let search = body["variables"]["search"].as_str().unwrap_or("");
                    if search.contains("Unmatched") {
                        return axum::Json(serde_json::json!({
                            "data": {
                                "Media": {
                                    "id": 999999,
                                    "title": { "romaji": "Completely Different Title" },
                                    "seasonYear": 1999
                                }
                            }
                        }));
                    }
                    axum::Json(serde_json::json!({
                        "data": {
                            "Media": {
                                "id": 918,
                                "idMal": 918,
                                "title": { "romaji": "Gintama", "english": "Gintama", "native": "銀魂" },
                                "description": "Odd Jobs Gin in Edo.",
                                "seasonYear": 2006,
                                "averageScore": 89.0,
                                "genres": ["Action", "Comedy"],
                                "coverImage": { "extraLarge": "https://cdn.anilist.example/918.jpg" },
                                "bannerImage": "https://cdn.anilist.example/918-banner.jpg",
                                "episodes": 201
                            }
                        }
                    }))
                }),
            )
            .route(
                "/tmdb/search/tv",
                get(|| async {
                    axum::Json(serde_json::json!({
                        "results": [{
                            "id": 1396,
                            "name": "Breaking Bad",
                            "first_air_date": "2008-01-20"
                        }]
                    }))
                }),
            )
            .route(
                "/tmdb/tv/1396",
                get(|Query(q): Query<TmdbQuery>| async move {
                    let overview = if q.language.as_deref() == Some("tr-TR") {
                        "Kimya öğretmeni Walter White'ın hikayesi."
                    } else {
                        "A chemistry teacher diagnosed with cancer."
                    };
                    axum::Json(serde_json::json!({
                        "id": 1396,
                        "name": "Breaking Bad",
                        "original_name": "Breaking Bad",
                        "overview": overview,
                        "poster_path": "/bb.jpg",
                        "backdrop_path": "/bb-bg.jpg",
                        "vote_average": 9.5,
                        "first_air_date": "2008-01-20",
                        "genres": [{"name": "Drama"}, {"name": "Crime"}],
                        "seasons": [{"season_number": 1, "episode_count": 7}],
                        "external_ids": {"imdb_id": "tt0903747", "tvdb_id": 81189}
                    }))
                }),
            )
            .route(
                "/cinemeta/catalog/movie/top/search=Interstellar.json",
                get(|| async {
                    axum::Json(serde_json::json!({
                        "metas": [{
                            "id": "tt0816692",
                            "name": "Interstellar",
                            "year": 2014
                        }]
                    }))
                }),
            )
            .route(
                "/cinemeta/meta/movie/tt0816692.json",
                get(|| async {
                    axum::Json(serde_json::json!({
                        "meta": {
                            "id": "tt0816692",
                            "name": "Interstellar",
                            "year": "2014",
                            "description": "A team travels through a wormhole.",
                            "poster": "https://cinemeta.example/interstellar.jpg",
                            "imdbRating": "8.7",
                            "genres": ["Sci-Fi", "Drama"]
                        }
                    }))
                }),
            );

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        MetaEndpoints {
            anilist_graphql: format!("{base}/anilist"),
            tmdb_api_base: format!("{base}/tmdb"),
            tmdb_image_base: "https://image.tmdb.org/t/p".into(),
            cinemeta_base: format!("{base}/cinemeta"),
        }
    }

    fn base_detail(title: &str, media_type: MediaType, is_anime: bool) -> MediaDetail {
        MediaDetail {
            native_id: "/item/1".into(),
            plugin_id: Some(PluginId::new("anthology").expect("id")),
            media_type,
            is_anime,
            title: title.into(),
            original_title: None,
            year: None,
            synopsis: None,
            poster: None,
            backdrop: None,
            genres: Vec::new(),
            rating: None,
            status: None,
            content_rating: ContentRating::Sfw,
            external_ids: ExternalIds::default(),
            seasons: None,
            episodes: vec![Episode {
                stream_id: "immutable-stream-id".into(),
                season: 1,
                episode: 1,
                absolute_episode: None,
                title: None,
                overview: None,
                thumbnail: None,
                air_date: None,
                tags: vec![EpisodeTag::Sub],
            }],
            relations: Vec::new(),
        }
    }

    #[tokio::test]
    async fn enriches_anime_tmdb_and_cinemeta_while_respecting_confidence_and_privacy_toggle() {
        let endpoints = spawn_mock_metadata_servers().await;
        let service = MetadataService::new(endpoints);

        // 1. Offline/privacy toggle suppresses all network enrichment.
        let gintama_raw = MediaDetail {
            year: Some(2006),
            ..base_detail("Gintama", MediaType::Series, true)
        };
        let offline = service
            .enrich_detail(&gintama_raw, "tr", None, false)
            .await
            .expect("offline no-op");
        assert!(offline.canonical.is_none());
        assert_eq!(offline.detail.poster, None);

        // 2. Anime heuristic match hits AniList GraphQL.
        let anime_out = service
            .enrich_detail(&gintama_raw, "tr", None, true)
            .await
            .expect("enrich anime");
        let anime_canon = anime_out.canonical.expect("anilist canonical");
        assert_eq!(anime_canon.canonical_id, "anilist:918");
        assert_eq!(anime_out.detail.external_ids.mal, Some(918));
        assert_eq!(
            anime_out.detail.episodes[0].stream_id,
            "immutable-stream-id"
        );

        // 3. Low-confidence mismatch degrades gracefully without touching poster/title.
        let bad_raw = base_detail("Unmatched Obscure Show", MediaType::Series, true);
        let unmatched = service
            .enrich_detail(&bad_raw, "en", None, true)
            .await
            .expect("graceful skip");
        assert!(unmatched.canonical.is_none());

        // 4. Series with TMDB key fetches localized Turkish overview + IMDB/TVDB cross-IDs.
        let bb_raw = MediaDetail {
            year: Some(2008),
            ..base_detail("Breaking Bad", MediaType::Series, false)
        };
        let bb_out = service
            .enrich_detail(&bb_raw, "tr", Some("test-tmdb-key"), true)
            .await
            .expect("enrich tmdb");
        assert_eq!(
            bb_out.detail.synopsis.as_deref(),
            Some("Kimya öğretmeni Walter White'ın hikayesi.")
        );
        assert_eq!(
            bb_out.detail.external_ids.imdb.as_deref(),
            Some("tt0903747")
        );

        // 5. Keyless movie falls back cleanly to Cinemeta JSON.
        let movie_raw = MediaDetail {
            year: Some(2014),
            ..base_detail("Interstellar", MediaType::Movie, false)
        };
        let movie_out = service
            .enrich_detail(&movie_raw, "en", None, true)
            .await
            .expect("enrich cinemeta");
        assert_eq!(
            movie_out.canonical.expect("cinemeta").canonical_id,
            "imdb:tt0816692"
        );
        assert_eq!(
            movie_out.detail.poster.as_deref(),
            Some("https://cinemeta.example/interstellar.jpg")
        );
    }
}
