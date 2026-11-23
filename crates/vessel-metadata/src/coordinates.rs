//! `anthology#10` bidirectional episode coordinate converter & non-destructive alignment.
//!
//! Converts between multi-season `(season, episode)` numbering (TMDB/TVDB) and single-season
//! absolute numbering (Anizium/AniList) so watch progress and subtitles stay synchronized across
//! heterogeneous plugins without ever rewriting a provider's own `stream_id` or episode list.

use serde::{Deserialize, Serialize};
use vessel_domain::Episode;

/// Per-season episode count descriptor used to translate `(season, episode) <-> absolute`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeasonSpan {
    pub season: u32,
    pub episode_count: u32,
}

/// Canonical episode metadata fetched from an external provider (never carries `stream_id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalEpisodeMeta {
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
}

/// Converts a `(season, episode)` coordinate into a 1-based `absolute_episode` across `spans`.
///
/// Ignores special season `0` when summing regular seasons (`season >= 1`).
pub fn season_episode_to_absolute(spans: &[SeasonSpan], season: u32, episode: u32) -> Option<u32> {
    if season == 0 || episode == 0 {
        return None;
    }

    let mut offset = 0u32;
    let mut ordered: Vec<SeasonSpan> = spans.iter().copied().filter(|s| s.season > 0).collect();
    ordered.sort_by_key(|s| s.season);

    for span in ordered {
        if span.season == season {
            if episode <= span.episode_count {
                return Some(offset + episode);
            }
            return None;
        }
        offset = offset.saturating_add(span.episode_count);
    }

    None
}

/// Converts a 1-based `absolute_episode` coordinate back into `(season, episode)`.
pub fn absolute_to_season_episode(spans: &[SeasonSpan], absolute: u32) -> Option<(u32, u32)> {
    if absolute == 0 {
        return None;
    }

    let mut ordered: Vec<SeasonSpan> = spans.iter().copied().filter(|s| s.season > 0).collect();
    ordered.sort_by_key(|s| s.season);

    let mut remaining = absolute;
    for span in ordered {
        if remaining <= span.episode_count {
            return Some((span.season, remaining));
        }
        remaining = remaining.saturating_sub(span.episode_count);
    }

    None
}

/// Enriches provider `episodes` non-destructively using `canonical_spans` and `canonical_episodes`.
///
/// Guarantees:
/// - `episodes.len()`, ordering, `stream_id`, and `tags` are NEVER modified.
/// - Sets missing `absolute_episode` whenever `(season, episode)` maps cleanly.
/// - Matches episode artwork/overview by `(season, episode)` first, then falls back to
///   `absolute_episode` (or resolved `S1E50 -> S2E1` coordinate) so single-season anime sites stay aligned.
pub fn align_episode_metadata(
    episodes: &mut [Episode],
    canonical_spans: &[SeasonSpan],
    canonical_episodes: &[CanonicalEpisodeMeta],
) {
    let plugin_max_season = episodes.iter().map(|e| e.season).max().unwrap_or(0);
    let canonical_max_season = canonical_spans
        .iter()
        .map(|s| s.season)
        .max()
        .or_else(|| canonical_episodes.iter().map(|e| e.season).max())
        .unwrap_or(0);

    // Detect single-season absolute layout (e.g. Anizium S1E1..S1E201 vs TMDB S1..S4).
    let plugin_uses_flat_s1 = plugin_max_season <= 1 && canonical_max_season > 1;

    for ep in episodes.iter_mut() {
        if ep.absolute_episode.is_none() {
            ep.absolute_episode = if plugin_uses_flat_s1 && ep.season == 1 {
                Some(ep.episode)
            } else {
                season_episode_to_absolute(canonical_spans, ep.season, ep.episode)
            };
        }

        let matched = canonical_episodes
            .iter()
            .find(|meta| {
                !plugin_uses_flat_s1 && meta.season == ep.season && meta.episode == ep.episode
            })
            .or_else(|| {
                ep.absolute_episode.and_then(|abs| {
                    canonical_episodes
                        .iter()
                        .find(|meta| meta.absolute_episode == Some(abs))
                        .or_else(|| {
                            let (s, e) = absolute_to_season_episode(canonical_spans, abs)?;
                            canonical_episodes
                                .iter()
                                .find(|meta| meta.season == s && meta.episode == e)
                        })
                })
            });

        if let Some(meta) = matched {
            if ep.title.as_deref().is_none_or(|t| t.trim().is_empty()) {
                ep.title = meta.title.clone();
            }
            if ep.overview.as_deref().is_none_or(|o| o.trim().is_empty()) {
                ep.overview = meta.overview.clone();
            }
            if ep.thumbnail.is_none() {
                ep.thumbnail = meta.thumbnail.clone();
            }
            if ep.air_date.is_none() {
                ep.air_date = meta.air_date.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use vessel_domain::{Episode, EpisodeTag};

    use super::{
        CanonicalEpisodeMeta, SeasonSpan, absolute_to_season_episode, align_episode_metadata,
        season_episode_to_absolute,
    };

    #[test]
    fn converts_gintama_coordinates_bidirectionally() {
        // Season 1 = 49 eps, Season 2 = 50 eps -> Gintama S2E1 == absolute 50.
        let spans = [
            SeasonSpan {
                season: 1,
                episode_count: 49,
            },
            SeasonSpan {
                season: 2,
                episode_count: 50,
            },
        ];

        assert_eq!(season_episode_to_absolute(&spans, 1, 49), Some(49));
        assert_eq!(season_episode_to_absolute(&spans, 2, 1), Some(50));
        assert_eq!(season_episode_to_absolute(&spans, 2, 51), None);

        assert_eq!(absolute_to_season_episode(&spans, 49), Some((1, 49)));
        assert_eq!(absolute_to_season_episode(&spans, 50), Some((2, 1)));
        assert_eq!(absolute_to_season_episode(&spans, 100), None);
    }

    #[test]
    fn aligns_flat_single_season_episodes_without_mutating_stream_ids() {
        let spans = [
            SeasonSpan {
                season: 1,
                episode_count: 49,
            },
            SeasonSpan {
                season: 2,
                episode_count: 50,
            },
        ];
        let canonical = vec![CanonicalEpisodeMeta {
            season: 2,
            episode: 1,
            absolute_episode: None,
            title: Some("Benizakura Begins".into()),
            overview: Some("Season 2 opener".into()),
            thumbnail: Some("https://img.example/s2e1.jpg".into()),
            air_date: Some("2007-04-05".into()),
        }];

        let mut plugin_eps = vec![Episode {
            stream_id: "anizium-ep-50".into(),
            season: 1,
            episode: 50,
            absolute_episode: None,
            title: None,
            overview: None,
            thumbnail: None,
            air_date: None,
            tags: vec![EpisodeTag::Sub],
        }];

        align_episode_metadata(&mut plugin_eps, &spans, &canonical);

        assert_eq!(plugin_eps.len(), 1);
        assert_eq!(plugin_eps[0].stream_id, "anizium-ep-50");
        assert_eq!(plugin_eps[0].tags, vec![EpisodeTag::Sub]);
        assert_eq!(plugin_eps[0].absolute_episode, Some(50));
        assert_eq!(plugin_eps[0].title.as_deref(), Some("Benizakura Begins"));
        assert_eq!(
            plugin_eps[0].thumbnail.as_deref(),
            Some("https://img.example/s2e1.jpg")
        );
    }
}
