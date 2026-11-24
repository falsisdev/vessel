//! User library watchlist and dual-coordinate watch progress persistence.

use std::collections::HashSet;

use rusqlite::{OptionalExtension, Row};
use serde::Serialize;
use vessel_domain::{
    LibraryItem, LibraryStatus, MediaRef, MediaType, PlaybackProgress, PluginId, VesselError,
    VesselResult,
};

use crate::{SqliteStorage, map_sql_err};

pub(crate) fn media_type_as_str(kind: MediaType) -> &'static str {
    match kind {
        MediaType::Movie => "movie",
        MediaType::Series => "series",
        MediaType::Live => "live",
    }
}

pub(crate) fn parse_media_type(raw: &str) -> VesselResult<MediaType> {
    match raw {
        "movie" => Ok(MediaType::Movie),
        "series" => Ok(MediaType::Series),
        "live" => Ok(MediaType::Live),
        other => Err(VesselError::Storage(format!(
            "unknown media_type '{other}'"
        ))),
    }
}

pub(crate) fn library_status_as_str(status: LibraryStatus) -> &'static str {
    match status {
        LibraryStatus::Watching => "watching",
        LibraryStatus::PlanToWatch => "plan_to_watch",
        LibraryStatus::Completed => "completed",
        LibraryStatus::OnHold => "on_hold",
        LibraryStatus::Dropped => "dropped",
    }
}

pub(crate) fn parse_library_status(raw: &str) -> VesselResult<LibraryStatus> {
    match raw {
        "watching" => Ok(LibraryStatus::Watching),
        "plan_to_watch" => Ok(LibraryStatus::PlanToWatch),
        "completed" => Ok(LibraryStatus::Completed),
        "on_hold" => Ok(LibraryStatus::OnHold),
        "dropped" => Ok(LibraryStatus::Dropped),
        other => Err(VesselError::Storage(format!(
            "unknown library status '{other}'"
        ))),
    }
}

pub(crate) fn u64_to_i64(value: u64, field: &str) -> VesselResult<i64> {
    i64::try_from(value).map_err(|_| VesselError::InvalidInput(format!("{field} overflows i64")))
}

pub(crate) fn i64_to_u64(value: i64, field: &str) -> VesselResult<u64> {
    u64::try_from(value).map_err(|_| VesselError::Storage(format!("negative {field} in database")))
}

pub(crate) struct RawLibraryRow {
    plugin_id: String,
    native_id: String,
    canonical_id: Option<String>,
    media_type_raw: String,
    is_anime: i64,
    title: String,
    poster_url: Option<String>,
    year: Option<u16>,
    status_raw: String,
    added_at: String,
    updated_at: String,
}

impl RawLibraryRow {
    pub(crate) fn from_sql(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            plugin_id: row.get(0)?,
            native_id: row.get(1)?,
            canonical_id: row.get(2)?,
            media_type_raw: row.get(3)?,
            is_anime: row.get(4)?,
            title: row.get(5)?,
            poster_url: row.get(6)?,
            year: row.get(7)?,
            status_raw: row.get(8)?,
            added_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }

    pub(crate) fn into_domain(self) -> VesselResult<LibraryItem> {
        let plugin_id = PluginId::new(self.plugin_id)?;
        let media = MediaRef::new(plugin_id, self.native_id)?;
        Ok(LibraryItem {
            media,
            canonical_id: self.canonical_id,
            media_type: parse_media_type(&self.media_type_raw)?,
            is_anime: self.is_anime != 0,
            title: self.title,
            poster_url: self.poster_url,
            year: self.year,
            status: parse_library_status(&self.status_raw)?,
            added_at: self.added_at,
            updated_at: self.updated_at,
        })
    }
}

pub(crate) struct RawProgressRow {
    stream_id: String,
    canonical_id: Option<String>,
    absolute_episode: Option<u32>,
    title: Option<String>,
    poster_url: Option<String>,
    backdrop_url: Option<String>,
    episode_title: Option<String>,
    position_ms: i64,
    duration_ms: i64,
    completed: i64,
    last_watched_at: String,
}

impl RawProgressRow {
    pub(crate) fn from_sql(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            stream_id: row.get(0)?,
            canonical_id: row.get(1)?,
            absolute_episode: row.get(2)?,
            position_ms: row.get(3)?,
            duration_ms: row.get(4)?,
            completed: row.get(5)?,
            last_watched_at: row.get(6)?,
            title: row.get(7)?,
            poster_url: row.get(8)?,
            backdrop_url: row.get(9)?,
            episode_title: row.get(10)?,
        })
    }

    pub(crate) fn into_domain(
        self,
        media: MediaRef,
        season: u32,
        episode: u32,
    ) -> VesselResult<PlaybackProgress> {
        Ok(PlaybackProgress {
            media,
            stream_id: self.stream_id,
            canonical_id: self.canonical_id,
            season,
            episode,
            absolute_episode: self.absolute_episode,
            title: self.title,
            poster_url: self.poster_url,
            backdrop_url: self.backdrop_url,
            episode_title: self.episode_title,
            position_ms: i64_to_u64(self.position_ms, "position_ms")?,
            duration_ms: i64_to_u64(self.duration_ms, "duration_ms")?,
            completed: self.completed != 0,
            last_watched_at: self.last_watched_at,
        })
    }
}

impl SqliteStorage {
    /// Lists library items, optionally filtered by watchlist status, ordered by `updated_at DESC`.
    pub async fn list_library_items(
        &self,
        status: Option<LibraryStatus>,
    ) -> VesselResult<Vec<LibraryItem>> {
        let status_filter = status.map(library_status_as_str);

        self.actor()
            .execute(move |db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT plugin_id, native_id, canonical_id, media_type, is_anime,
                                title, poster_url, year, status, added_at, updated_at
                         FROM library_items
                         WHERE (?1 IS NULL OR status = ?1)
                         ORDER BY updated_at DESC",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([status_filter], RawLibraryRow::from_sql)
                    .map_err(map_sql_err)?;

                let mut items = Vec::new();
                for row in rows {
                    items.push(row.map_err(map_sql_err)?.into_domain()?);
                }
                Ok(items)
            })
            .await
    }

    /// Inserts or updates a library item keyed by `(plugin_id, native_id)`.
    pub async fn upsert_library(&self, item: &LibraryItem) -> VesselResult<()> {
        let row_id = format!("{}:{}", item.media.plugin_id, item.media.native_id);
        let item = item.clone();
        let media_type = media_type_as_str(item.media_type);
        let status = library_status_as_str(item.status);

        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO library_items (
                             id, plugin_id, native_id, canonical_id, media_type, is_anime,
                             title, poster_url, year, status, added_at, updated_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                         ON CONFLICT(plugin_id, native_id) DO UPDATE SET
                             canonical_id = excluded.canonical_id,
                             media_type = excluded.media_type,
                             is_anime = excluded.is_anime,
                             title = excluded.title,
                             poster_url = excluded.poster_url,
                             year = excluded.year,
                             status = excluded.status,
                             updated_at = excluded.updated_at",
                        (
                            &row_id,
                            item.media.plugin_id.as_str(),
                            &item.media.native_id,
                            &item.canonical_id,
                            media_type,
                            i32::from(item.is_anime),
                            &item.title,
                            &item.poster_url,
                            item.year,
                            status,
                            &item.added_at,
                            &item.updated_at,
                        ),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Deletes a library item matching `(plugin_id, native_id)`.
    pub async fn delete_library_item(&self, media: &MediaRef) -> VesselResult<()> {
        let plugin_id = media.plugin_id.as_str().to_owned();
        let native_id = media.native_id.clone();

        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "DELETE FROM library_items WHERE plugin_id = ?1 AND native_id = ?2",
                        (&plugin_id, &native_id),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Reads watch progress for a specific `(plugin_id, native_id, season, episode)`.
    pub async fn get_watch_progress(
        &self,
        media: &MediaRef,
        season: u32,
        episode: u32,
    ) -> VesselResult<Option<PlaybackProgress>> {
        let media = media.clone();

        self.actor()
            .execute(move |db| {
                let row = db
                    .conn()
                    .query_row(
                        "SELECT stream_id, canonical_id, absolute_episode, position_ms,
                                duration_ms, completed, last_watched_at,
                                title, poster_url, backdrop_url, episode_title
                         FROM watch_progress
                         WHERE plugin_id = ?1 AND native_id = ?2 AND season = ?3 AND episode = ?4",
                        (media.plugin_id.as_str(), &media.native_id, season, episode),
                        RawProgressRow::from_sql,
                    )
                    .optional()
                    .map_err(map_sql_err)?;

                row.map(|raw| raw.into_domain(media, season, episode))
                    .transpose()
            })
            .await
    }

    /// Reads the most recent cross-plugin watch progress matching `(canonical_id, absolute_episode)`.
    ///
    /// Preserves playback continuity across providers with incompatible season layouts
    /// (e.g. `Gintama S2E1` vs `Anizium S1E50`).
    pub async fn get_watch_progress_by_canonical(
        &self,
        canonical_id: &str,
        absolute_episode: u32,
    ) -> VesselResult<Option<PlaybackProgress>> {
        let canonical_id = canonical_id.trim().to_owned();
        if canonical_id.is_empty() || absolute_episode == 0 {
            return Ok(None);
        }

        self.actor()
            .execute(move |db| {
                let row = db
                    .conn()
                    .query_row(
                        "SELECT plugin_id, native_id, season, episode,
                                stream_id, canonical_id, absolute_episode, position_ms,
                                duration_ms, completed, last_watched_at,
                                title, poster_url, backdrop_url, episode_title
                         FROM watch_progress
                         WHERE canonical_id = ?1 AND absolute_episode = ?2
                         ORDER BY last_watched_at DESC
                         LIMIT 1",
                        (&canonical_id, absolute_episode),
                        |row| {
                            let plugin_raw: String = row.get(0)?;
                            let native_id: String = row.get(1)?;
                            let season: u32 = row.get(2)?;
                            let episode: u32 = row.get(3)?;
                            let raw = RawProgressRow {
                                stream_id: row.get(4)?,
                                canonical_id: row.get(5)?,
                                absolute_episode: row.get(6)?,
                                position_ms: row.get(7)?,
                                duration_ms: row.get(8)?,
                                completed: row.get(9)?,
                                last_watched_at: row.get(10)?,
                                title: row.get(11)?,
                                poster_url: row.get(12)?,
                                backdrop_url: row.get(13)?,
                                episode_title: row.get(14)?,
                            };
                            Ok((plugin_raw, native_id, season, episode, raw))
                        },
                    )
                    .optional()
                    .map_err(map_sql_err)?;

                match row {
                    Some((plugin_raw, native_id, season, episode, raw)) => {
                        let plugin_id = PluginId::new(plugin_raw)?;
                        let media = MediaRef::new(plugin_id, native_id)?;
                        Ok(Some(raw.into_domain(media, season, episode)?))
                    }
                    None => Ok(None),
                }
            })
            .await
    }

    /// Inserts or updates watch progress for `(plugin_id, native_id, season, episode)`.
    pub async fn upsert_watch_progress(&self, progress: &PlaybackProgress) -> VesselResult<()> {
        let row_id = format!(
            "{}:{}:{}:{}",
            progress.media.plugin_id, progress.media.native_id, progress.season, progress.episode
        );
        let position_ms = u64_to_i64(progress.position_ms, "position_ms")?;
        let duration_ms = u64_to_i64(progress.duration_ms, "duration_ms")?;
        let progress = progress.clone();

        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO watch_progress (
                             id, plugin_id, native_id, stream_id, canonical_id, season, episode,
                             absolute_episode, position_ms, duration_ms, completed, last_watched_at,
                             title, poster_url, backdrop_url, episode_title
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                         ON CONFLICT(plugin_id, native_id, season, episode) DO UPDATE SET
                             stream_id = excluded.stream_id,
                             canonical_id = excluded.canonical_id,
                             absolute_episode = excluded.absolute_episode,
                             position_ms = excluded.position_ms,
                             duration_ms = excluded.duration_ms,
                             completed = excluded.completed,
                             last_watched_at = excluded.last_watched_at,
                             title = COALESCE(NULLIF(excluded.title, ''), watch_progress.title),
                             poster_url = COALESCE(NULLIF(excluded.poster_url, ''), watch_progress.poster_url),
                             backdrop_url = COALESCE(NULLIF(excluded.backdrop_url, ''), watch_progress.backdrop_url),
                             episode_title = COALESCE(NULLIF(excluded.episode_title, ''), watch_progress.episode_title)",
                        rusqlite::params![
                            &row_id,
                            progress.media.plugin_id.as_str(),
                            &progress.media.native_id,
                            &progress.stream_id,
                            &progress.canonical_id,
                            progress.season,
                            progress.episode,
                            progress.absolute_episode,
                            position_ms,
                            duration_ms,
                            i32::from(progress.completed),
                            &progress.last_watched_at,
                            &progress.title,
                            &progress.poster_url,
                            &progress.backdrop_url,
                            &progress.episode_title,
                        ],
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Lists all watch progress rows for a given `(plugin_id, native_id)` ordered by `(season, episode)`.
    pub async fn list_media_watch_progress(
        &self,
        media: &MediaRef,
    ) -> VesselResult<Vec<PlaybackProgress>> {
        let plugin_raw = media.plugin_id.as_str().to_owned();
        let native_id = media.native_id.clone();

        self.actor()
            .execute(move |db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT stream_id, canonical_id, absolute_episode,
                                position_ms, duration_ms, completed, last_watched_at,
                                title, poster_url, backdrop_url, episode_title,
                                season, episode
                         FROM watch_progress
                         WHERE plugin_id = ?1 AND native_id = ?2
                         ORDER BY season ASC, episode ASC",
                    )
                    .map_err(map_sql_err)?;

                let rows = stmt
                    .query_map((&plugin_raw, &native_id), |row| {
                        let raw = RawProgressRow::from_sql(row)?;
                        let season: u32 = row.get(11)?;
                        let episode: u32 = row.get(12)?;
                        Ok((raw, season, episode))
                    })
                    .map_err(map_sql_err)?;

                let mut items = Vec::new();
                for row in rows {
                    let (raw, season, episode) = row.map_err(map_sql_err)?;
                    let media_ref =
                        MediaRef::new(PluginId::new(plugin_raw.clone())?, native_id.clone())?;
                    items.push(raw.into_domain(media_ref, season, episode)?);
                }
                Ok(items)
            })
            .await
    }

    /// Lists recent watch progress rows across all media ordered newest-first.
    pub async fn list_all_watch_progress(
        &self,
        limit: usize,
    ) -> VesselResult<Vec<PlaybackProgress>> {
        let cap = limit.clamp(1, 500) as i64;
        self.actor()
            .execute(move |db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT stream_id, canonical_id, absolute_episode,
                                position_ms, duration_ms, completed, last_watched_at,
                                title, poster_url, backdrop_url, episode_title,
                                plugin_id, native_id, season, episode
                         FROM watch_progress
                         ORDER BY last_watched_at DESC
                         LIMIT ?1",
                    )
                    .map_err(map_sql_err)?;

                let rows = stmt
                    .query_map([cap], |row| {
                        let raw = RawProgressRow::from_sql(row)?;
                        let plugin_raw: String = row.get(11)?;
                        let native_id: String = row.get(12)?;
                        let season: u32 = row.get(13)?;
                        let episode: u32 = row.get(14)?;
                        Ok((raw, plugin_raw, native_id, season, episode))
                    })
                    .map_err(map_sql_err)?;

                let mut items = Vec::new();
                for row in rows {
                    let (raw, plugin_raw, native_id, season, episode) = row.map_err(map_sql_err)?;
                    let media_ref = MediaRef::new(PluginId::new(plugin_raw)?, native_id)?;
                    items.push(raw.into_domain(media_ref, season, episode)?);
                }
                Ok(items)
            })
            .await
    }

    /// Lists recently watched incomplete or up-next items (deduplicated per show/movie, newest first).
    pub async fn list_continue_watching(
        &self,
        limit: usize,
    ) -> VesselResult<Vec<ContinueWatchingEntry>> {
        let cap = limit.clamp(1, 50);
        self.actor()
            .execute(move |db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        r"SELECT
                             wp.plugin_id, wp.native_id, wp.stream_id, wp.canonical_id,
                             wp.season, wp.episode, wp.absolute_episode,
                             wp.position_ms, wp.duration_ms, wp.completed, wp.last_watched_at,
                             wp.title, wp.poster_url, wp.backdrop_url, wp.episode_title,
                             li.title, li.poster_url, li.media_type
                         FROM watch_progress wp
                         LEFT JOIN library_items li
                           ON li.plugin_id = wp.plugin_id AND li.native_id = wp.native_id
                         WHERE (wp.completed = 1 OR wp.position_ms > 0)
                           AND COALESCE(li.media_type, '') != 'live'
                           AND wp.stream_id NOT LIKE 'tv:%'
                           AND wp.stream_id NOT LIKE 'tv\_%' ESCAPE '\'
                           AND wp.stream_id NOT LIKE 'iptv\_%' ESCAPE '\'
                           AND wp.stream_id NOT LIKE 'vns::tv::%'
                         ORDER BY wp.last_watched_at DESC
                         LIMIT ?1",
                    )
                    .map_err(map_sql_err)?;

                let raw_limit = (cap * 6) as i64;
                let mut rows = stmt.query([raw_limit]).map_err(map_sql_err)?;
                let mut seen: HashSet<(String, String)> = HashSet::new();
                let mut out = Vec::new();

                while let Some(row) = rows.next().map_err(map_sql_err)? {
                    let plugin_raw: String = row.get(0).map_err(map_sql_err)?;
                    let native_id: String = row.get(1).map_err(map_sql_err)?;
                    if !seen.insert((plugin_raw.clone(), native_id.clone())) {
                        continue;
                    }

                    let mut stream_id: String = row.get(2).map_err(map_sql_err)?;
                    let canonical_id: Option<String> = row.get(3).map_err(map_sql_err)?;
                    let mut season: u32 = row.get(4).map_err(map_sql_err)?;
                    let mut episode: u32 = row.get(5).map_err(map_sql_err)?;
                    let mut absolute_episode: Option<u32> = row.get(6).map_err(map_sql_err)?;
                    let mut position_ms = row.get::<_, i64>(7).map_err(map_sql_err)?.max(0) as u64;
                    let mut duration_ms = row.get::<_, i64>(8).map_err(map_sql_err)?.max(0) as u64;
                    let mut completed = row.get::<_, i32>(9).map_err(map_sql_err)? != 0;
                    let last_watched_at: String = row.get(10).map_err(map_sql_err)?;

                    let wp_title: Option<String> = row.get(11).map_err(map_sql_err)?;
                    let wp_poster: Option<String> = row.get(12).map_err(map_sql_err)?;
                    let mut backdrop: Option<String> = row.get(13).map_err(map_sql_err)?;
                    let mut episode_title: Option<String> = row.get(14).map_err(map_sql_err)?;
                    let li_title: Option<String> = row.get(15).map_err(map_sql_err)?;
                    let li_poster: Option<String> = row.get(16).map_err(map_sql_err)?;
                    let mut media_type = row
                        .get::<_, Option<String>>(17)
                        .map_err(map_sql_err)?
                        .and_then(|s| parse_media_type(&s).ok())
                        .unwrap_or(if season == 0 && episode == 0 {
                            MediaType::Movie
                        } else {
                            MediaType::Series
                        });

                    let mut title = wp_title
                        .filter(|s| !s.trim().is_empty())
                        .or_else(|| li_title.filter(|s| !s.trim().is_empty()));
                    let mut poster = wp_poster
                        .filter(|s| !s.trim().is_empty())
                        .or_else(|| li_poster.filter(|s| !s.trim().is_empty()));

                    let episodes_key = format!("episodes:{plugin_raw}:{native_id}");
                    let meta3_pattern = format!("meta3:{plugin_raw}:{native_id}:%");
                    let meta2_pattern = format!("meta2:{plugin_raw}:{native_id}:%");
                    let meta_pattern = format!("meta:{plugin_raw}:{native_id}:%");
                    let detail_pattern = format!("detail:{plugin_raw}:{native_id}:%");

                    let mut cached_episodes: Option<Vec<serde_json::Value>> = None;
                    let mut cache_stmt = db
                        .conn()
                        .prepare(
                            "SELECT cache_key, payload_json FROM metadata_cache
                             WHERE cache_key = ?1
                                OR cache_key LIKE ?2
                                OR cache_key LIKE ?3
                                OR cache_key LIKE ?4
                                OR cache_key LIKE ?5
                             ORDER BY CASE WHEN cache_key = ?1 THEN 0 ELSE 1 END ASC, expires_at DESC
                             LIMIT 6",
                        )
                        .map_err(map_sql_err)?;
                    let cache_rows = cache_stmt
                        .query_map(
                            (
                                &episodes_key,
                                &meta3_pattern,
                                &meta2_pattern,
                                &meta_pattern,
                                &detail_pattern,
                            ),
                            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                        )
                        .map_err(map_sql_err)?;

                    for cache_row in cache_rows {
                        let (_, json_str) = cache_row.map_err(map_sql_err)?;
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            if title.is_none() {
                                title = val
                                    .get("title")
                                    .and_then(|v| v.as_str())
                                    .filter(|s| !s.trim().is_empty())
                                    .map(str::to_owned);
                            }
                            if poster.is_none() {
                                poster = val
                                    .get("poster")
                                    .and_then(|v| v.as_str())
                                    .filter(|s| !s.trim().is_empty())
                                    .map(str::to_owned);
                            }
                            if backdrop.as_ref().is_none_or(|b| b.trim().is_empty()) {
                                backdrop = val
                                    .get("backdrop")
                                    .and_then(|v| v.as_str())
                                    .filter(|s| !s.trim().is_empty())
                                    .map(str::to_owned);
                            }
                            if let Some(t) = val.get("type").and_then(|v| v.as_str())
                                && let Ok(parsed) = parse_media_type(t)
                            {
                                media_type = parsed;
                            }
                            if cached_episodes.is_none()
                                && let Some(eps) = val.get("episodes").and_then(|v| v.as_array())
                                && !eps.is_empty()
                            {
                                cached_episodes = Some(eps.clone());
                            }
                        }
                    }

                    if media_type == MediaType::Live {
                        continue;
                    }

                    if completed {
                        if media_type == MediaType::Movie {
                            continue;
                        }
                        let Some(mut eps) = cached_episodes.clone() else {
                            continue;
                        };
                        eps.sort_by_key(|e| {
                            (
                                e.get("season").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
                                e.get("episode").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                            )
                        });
                        let current_idx = eps.iter().position(|e| {
                            e.get("streamId").and_then(|v| v.as_str()) == Some(stream_id.as_str())
                                || (e.get("season").and_then(|v| v.as_u64()).unwrap_or(1) as u32
                                    == season
                                    && e.get("episode").and_then(|v| v.as_u64()).unwrap_or(0)
                                        as u32
                                        == episode)
                        });
                        let next_ep = match current_idx {
                            Some(idx) => eps.get(idx + 1),
                            None => eps.iter().find(|e| {
                                let es =
                                    e.get("season").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                                let ee =
                                    e.get("episode").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                (es, ee) > (season, episode)
                            }),
                        };
                        let Some(next_val) = next_ep else {
                            // Final episode completed -> remove from Continue Watching while preserving DB progress.
                            continue;
                        };
                        let next_season = next_val
                            .get("season")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(1) as u32;
                        let next_episode = next_val
                            .get("episode")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;
                        let next_stream_id = next_val
                            .get("streamId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned();
                        if next_stream_id.is_empty() {
                            continue;
                        }
                        let next_existing: Option<(i64, i64, i32)> = db
                            .conn()
                            .query_row(
                                "SELECT position_ms, duration_ms, completed
                                 FROM watch_progress
                                 WHERE plugin_id = ?1 AND native_id = ?2 AND season = ?3 AND episode = ?4",
                                (&plugin_raw, &native_id, next_season, next_episode),
                                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                            )
                            .optional()
                            .map_err(map_sql_err)?;
                        if let Some((pos, dur, next_done)) = next_existing {
                            if next_done != 0 {
                                continue;
                            }
                            position_ms = pos.max(0) as u64;
                            duration_ms = dur.max(0) as u64;
                        } else {
                            position_ms = 0;
                            duration_ms = 0;
                        }
                        season = next_season;
                        episode = next_episode;
                        stream_id = next_stream_id;
                        absolute_episode = next_val
                            .get("absoluteEpisode")
                            .and_then(|v| v.as_u64())
                            .map(|n| n as u32);
                        episode_title = next_val
                            .get("title")
                            .and_then(|v| v.as_str())
                            .map(str::to_owned);
                        completed = false;
                    } else if episode_title.as_ref().is_none_or(|t| t.trim().is_empty())
                        && let Some(ref eps) = cached_episodes
                    {
                        episode_title = eps
                            .iter()
                            .find(|e| {
                                e.get("streamId").and_then(|v| v.as_str())
                                    == Some(stream_id.as_str())
                                    || (e.get("season").and_then(|v| v.as_u64())
                                        == Some(u64::from(season))
                                        && e.get("episode").and_then(|v| v.as_u64())
                                            == Some(u64::from(episode)))
                            })
                            .and_then(|e| e.get("title").and_then(|v| v.as_str()))
                            .map(str::to_owned);
                    }

                    let plugin_id = PluginId::new(plugin_raw)?;
                    out.push(ContinueWatchingEntry {
                        media: MediaRef {
                            plugin_id,
                            native_id: native_id.clone(),
                        },
                        title: title.unwrap_or(native_id),
                        poster,
                        backdrop,
                        media_type,
                        stream_id,
                        canonical_id,
                        season,
                        episode,
                        absolute_episode,
                        episode_title,
                        position_ms,
                        duration_ms,
                        completed,
                        last_watched_at,
                    });

                    if out.len() >= cap {
                        break;
                    }
                }

                Ok(out)
            })
            .await
    }

    /// Removes saved watch progress for `(plugin_id, native_id)` from Continue Watching.
    pub async fn delete_watch_progress(
        &self,
        plugin_id: &PluginId,
        native_id: &str,
    ) -> VesselResult<()> {
        let plugin = plugin_id.as_str().to_owned();
        let native = native_id.to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "DELETE FROM watch_progress WHERE plugin_id = ?1 AND native_id = ?2",
                        (&plugin, &native),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }
}

/// Enriched Continue Watching card entry backed by `watch_progress`, `library_items`, and `metadata_cache`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueWatchingEntry {
    pub media: MediaRef,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<String>,
    pub media_type: MediaType,
    pub stream_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_id: Option<String>,
    pub season: u32,
    pub episode: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub absolute_episode: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_title: Option<String>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub completed: bool,
    pub last_watched_at: String,
}
