//! User library watchlist and dual-coordinate watch progress persistence.

use rusqlite::{OptionalExtension, Row};
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
                                duration_ms, completed, last_watched_at
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
                             absolute_episode, position_ms, duration_ms, completed, last_watched_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                         ON CONFLICT(plugin_id, native_id, season, episode) DO UPDATE SET
                             stream_id = excluded.stream_id,
                             canonical_id = excluded.canonical_id,
                             absolute_episode = excluded.absolute_episode,
                             position_ms = excluded.position_ms,
                             duration_ms = excluded.duration_ms,
                             completed = excluded.completed,
                             last_watched_at = excluded.last_watched_at",
                        (
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
                        ),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }
}
