//! Non-destructive 4-step metadata merge rules (`PLAN.md` 3.3.B.3).

use serde::{Deserialize, Serialize};
use vessel_domain::{ExternalIds, MediaDetail, MediaType};

use crate::coordinates::{CanonicalEpisodeMeta, SeasonSpan, align_episode_metadata};

/// Normalized external metadata fetched from AniList, TMDB, or Cinemeta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalMetadata {
    pub canonical_id: String,
    pub external_ids: ExternalIds,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,
    /// BCP-47 / ISO 639-1 language tag of `synopsis` (e.g. `"tr"`, `"en"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synopsis_lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trailer_url: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f32>,
    #[serde(default)]
    pub season_spans: Vec<SeasonSpan>,
    #[serde(default)]
    pub episodes: Vec<CanonicalEpisodeMeta>,
}

/// Derives a stable cross-provider canonical identifier from known external IDs.
pub fn canonical_id_from_external_ids(ids: &ExternalIds, media_type: MediaType) -> Option<String> {
    if let Some(anilist) = ids.anilist {
        return Some(format!("anilist:{anilist}"));
    }
    if let Some(tmdb) = ids.tmdb {
        let kind = match media_type {
            MediaType::Movie => "movie",
            MediaType::Series | MediaType::Live => "tv",
        };
        return Some(format!("tmdb:{kind}:{tmdb}"));
    }
    if let Some(ref imdb) = ids.imdb {
        return Some(format!("imdb:{imdb}"));
    }
    if let Some(mal) = ids.mal {
        return Some(format!("mal:{mal}"));
    }
    if let Some(kitsu) = ids.kitsu {
        return Some(format!("kitsu:{kitsu}"));
    }
    if let Some(simkl) = ids.simkl {
        return Some(format!("simkl:{simkl}"));
    }
    None
}

/// Merges `enriched` metadata into a provider's raw `MediaDetail` without breaking playback.
///
/// - Preserves plugin-owned `native_id`, `plugin_id`, `episodes[]` count/order/stream IDs, and
///   anime `sub`/`dub` tags.
/// - Preserves plugin's own non-empty synopsis when user wants e.g. `"tr"` and external synopsis
///   is missing or only English when plugin already supplied localized text.
pub fn merge_detail(
    raw: &MediaDetail,
    enriched: &CanonicalMetadata,
    preferred_lang: &str,
) -> MediaDetail {
    merge_detail_with_override(raw, enriched, preferred_lang, false)
}

/// Merges `enriched` metadata into `raw`; when `force_replace` is `true` (manual Fix Match),
/// title, year, synopsis, and external IDs from the matched source overwrite the plugin defaults.
pub fn merge_detail_with_override(
    raw: &MediaDetail,
    enriched: &CanonicalMetadata,
    preferred_lang: &str,
    force_replace: bool,
) -> MediaDetail {
    let mut merged = raw.clone();

    if force_replace {
        if !enriched.title.trim().is_empty() {
            merged.title = enriched.title.clone();
        }
        merged.original_title = enriched.original_title.clone().or(merged.original_title);
        if enriched.year.is_some() {
            merged.year = enriched.year;
        }
    } else {
        if merged.original_title.is_none() {
            merged.original_title = enriched.original_title.clone();
        }
        if merged.year.is_none() {
            merged.year = enriched.year;
        }
    }

    if let Some(ref ext_poster) = enriched.poster {
        merged.poster = Some(ext_poster.clone());
    }
    if let Some(ref ext_backdrop) = enriched.backdrop {
        merged.backdrop = Some(ext_backdrop.clone());
    }
    if let Some(ref ext_trailer) = enriched.trailer_url
        && !ext_trailer.trim().is_empty()
    {
        merged.trailer_url = Some(ext_trailer.clone());
    }
    if !enriched.genres.is_empty() {
        merged.genres = enriched.genres.clone();
    }
    if enriched.rating.is_some() {
        merged.rating = enriched.rating;
    }

    if let Some(ref ext_synopsis) = enriched.synopsis
        && !ext_synopsis.trim().is_empty()
    {
        let raw_has_synopsis = raw
            .synopsis
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty());
        let ext_matches_pref = enriched
            .synopsis_lang
            .as_deref()
            .is_some_and(|lang| lang.eq_ignore_ascii_case(preferred_lang));
        if force_replace || !raw_has_synopsis || ext_matches_pref {
            merged.synopsis = Some(ext_synopsis.clone());
        }
    }

    if force_replace {
        merged.external_ids = enriched.external_ids.clone();
    } else {
        merged.external_ids.tmdb = merged.external_ids.tmdb.or(enriched.external_ids.tmdb);
        merged.external_ids.imdb = merged
            .external_ids
            .imdb
            .clone()
            .or_else(|| enriched.external_ids.imdb.clone());
        merged.external_ids.tvdb = merged.external_ids.tvdb.or(enriched.external_ids.tvdb);
        merged.external_ids.anilist = merged
            .external_ids
            .anilist
            .or(enriched.external_ids.anilist);
        merged.external_ids.kitsu = merged.external_ids.kitsu.or(enriched.external_ids.kitsu);
        merged.external_ids.mal = merged.external_ids.mal.or(enriched.external_ids.mal);
        merged.external_ids.simkl = merged.external_ids.simkl.or(enriched.external_ids.simkl);
    }

    // Clear cloned poster thumbnails on episodes so external episode stills replace show-poster placeholders.
    for ep in &mut merged.episodes {
        if let (Some(thumb), Some(raw_poster)) = (&ep.thumbnail, &raw.poster)
            && thumb == raw_poster
        {
            ep.thumbnail = None;
        }
    }

    align_episode_metadata(
        &mut merged.episodes,
        &enriched.season_spans,
        &enriched.episodes,
    );

    merged
}

#[cfg(test)]
mod tests {
    use vessel_domain::{
        ContentRating, Episode, EpisodeTag, ExternalIds, MediaDetail, MediaType, PluginId,
    };

    use super::{CanonicalMetadata, merge_detail};

    #[test]
    fn preserves_localized_synopsis_and_immutable_plugin_episodes() {
        let plugin_id = PluginId::new("anthology").expect("id");
        let raw = MediaDetail {
            native_id: "/anime/gintama".into(),
            plugin_id: Some(plugin_id),
            media_type: MediaType::Series,
            is_anime: true,
            title: "Gintama".into(),
            original_title: None,
            year: Some(2006),
            synopsis: Some("Edo döneminde samuray komedisi.".into()),
            poster: Some("https://plugin.example/lowres.jpg".into()),
            backdrop: None,
            trailer_url: None,
            genres: Vec::new(),
            rating: None,
            status: None,
            content_rating: ContentRating::Sfw,
            external_ids: ExternalIds::default(),
            seasons: None,
            episodes: vec![Episode {
                stream_id: "ep-stream-1".into(),
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
        };

        let enriched = CanonicalMetadata {
            canonical_id: "anilist:918".into(),
            external_ids: ExternalIds {
                anilist: Some(918),
                mal: Some(918),
                ..ExternalIds::default()
            },
            title: "Gintama".into(),
            original_title: Some("銀魂".into()),
            year: Some(2006),
            synopsis: Some("English fallback synopsis.".into()),
            synopsis_lang: Some("en".into()),
            poster: Some("https://anilist.example/4k.jpg".into()),
            backdrop: Some("https://anilist.example/banner.jpg".into()),
            trailer_url: Some("https://www.youtube.com/watch?v=918".into()),
            genres: vec!["Comedy".into(), "Action".into()],
            rating: Some(8.9),
            season_spans: Vec::new(),
            episodes: Vec::new(),
        };

        // Preferred language is "tr", so plugin's existing Turkish synopsis stays untouched while
        // poster/backdrop/genres/externalIds upgrade cleanly.
        let merged = merge_detail(&raw, &enriched, "tr");
        assert_eq!(
            merged.synopsis.as_deref(),
            Some("Edo döneminde samuray komedisi.")
        );
        assert_eq!(
            merged.poster.as_deref(),
            Some("https://anilist.example/4k.jpg")
        );
        assert_eq!(
            merged.backdrop.as_deref(),
            Some("https://anilist.example/banner.jpg")
        );
        assert_eq!(merged.external_ids.anilist, Some(918));
        assert_eq!(merged.episodes.len(), 1);
        assert_eq!(merged.episodes[0].stream_id, "ep-stream-1");
        assert_eq!(merged.episodes[0].tags, vec![EpisodeTag::Sub]);
    }
}
