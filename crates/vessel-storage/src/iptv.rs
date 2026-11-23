//! SQLite persistence for IPTV profiles, FTS5-indexed channels, favorites, and XMLTV EPG.

use std::collections::HashSet;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use vessel_domain::{VesselError, VesselResult};

use crate::{SqliteStorage, map_sql_err};

/// IPTV backend kind stored in `iptv_profiles.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredIptvKind {
    Xtream,
    M3u,
}

impl StoredIptvKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Xtream => "xtream",
            Self::M3u => "m3u",
        }
    }

    fn parse(raw: &str) -> VesselResult<Self> {
        match raw {
            "xtream" => Ok(Self::Xtream),
            "m3u" => Ok(Self::M3u),
            other => Err(VesselError::Storage(format!("unknown iptv kind '{other}'"))),
        }
    }
}

/// Persisted IPTV profile row matching `iptv_profiles`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredIptvProfile {
    pub id: String,
    pub name: String,
    pub kind: StoredIptvKind,
    pub server_url: String,
    pub username: Option<String>,
    pub secret_ref: Option<String>,
    pub epg_url: Option<String>,
    pub user_agent: Option<String>,
    pub last_synced_at: Option<String>,
}

/// Channel input batch inserted into `iptv_channels`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewIptvChannel {
    pub stream_id: String,
    pub name: String,
    pub group_title: String,
    pub logo_url: Option<String>,
    pub stream_url: String,
    pub tvg_id: Option<String>,
}

/// Persisted channel row matching `iptv_channels`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredIptvChannel {
    pub id: i64,
    pub profile_id: String,
    pub stream_id: String,
    pub name: String,
    pub group_title: String,
    pub logo_url: Option<String>,
    pub stream_url: String,
    pub tvg_id: Option<String>,
    pub is_favorite: bool,
}

/// EPG schedule item stored in `epg_programs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredEpgProgram {
    pub channel_tvg_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_ts: i64,
    pub end_ts: i64,
}

/// Now & Next EPG card snapshot for a single channel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredEpgNowNext {
    pub now: Option<StoredEpgProgram>,
    pub next: Option<StoredEpgProgram>,
}

fn build_fts_query(raw: &str) -> Option<String> {
    let terms: Vec<String> = raw
        .split_whitespace()
        .map(|tok| {
            tok.chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect::<String>()
        })
        .filter(|tok| !tok.is_empty())
        .map(|tok| format!("\"{tok}\"*"))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

fn channel_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredIptvChannel> {
    let fav: i64 = row.get(8)?;
    Ok(StoredIptvChannel {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        stream_id: row.get(2)?,
        name: row.get(3)?,
        group_title: row.get(4)?,
        logo_url: row.get(5)?,
        stream_url: row.get(6)?,
        tvg_id: row.get(7)?,
        is_favorite: fav != 0,
    })
}

impl SqliteStorage {
    /// Creates or updates an IPTV profile row.
    pub async fn upsert_iptv_profile(&self, profile: &StoredIptvProfile) -> VesselResult<()> {
        let p = profile.clone();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO iptv_profiles (
                             id, name, kind, server_url, username, secret_ref, epg_url, user_agent, last_synced_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                         ON CONFLICT(id) DO UPDATE SET
                             name = excluded.name,
                             kind = excluded.kind,
                             server_url = excluded.server_url,
                             username = excluded.username,
                             secret_ref = excluded.secret_ref,
                             epg_url = excluded.epg_url,
                             user_agent = excluded.user_agent,
                             last_synced_at = excluded.last_synced_at",
                        (
                            &p.id,
                            &p.name,
                            p.kind.as_str(),
                            &p.server_url,
                            &p.username,
                            &p.secret_ref,
                            &p.epg_url,
                            &p.user_agent,
                            &p.last_synced_at,
                        ),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Lists all configured IPTV profiles ordered by name.
    pub async fn list_iptv_profiles(&self) -> VesselResult<Vec<StoredIptvProfile>> {
        self.actor()
            .execute(|db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT id, name, kind, server_url, username, secret_ref, epg_url, user_agent, last_synced_at
                         FROM iptv_profiles ORDER BY name ASC",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, Option<String>>(8)?,
                        ))
                    })
                    .map_err(map_sql_err)?;

                let mut profiles = Vec::new();
                for item in rows {
                    let (id, name, kind_raw, server_url, username, secret_ref, epg_url, user_agent, last_synced_at) =
                        item.map_err(map_sql_err)?;
                    profiles.push(StoredIptvProfile {
                        id,
                        name,
                        kind: StoredIptvKind::parse(&kind_raw)?,
                        server_url,
                        username,
                        secret_ref,
                        epg_url,
                        user_agent,
                        last_synced_at,
                    });
                }
                Ok(profiles)
            })
            .await
    }

    /// Deletes an IPTV profile (cascading its channels, FTS rows, and EPG programs).
    pub async fn delete_iptv_profile(&self, profile_id: &str) -> VesselResult<()> {
        let id = profile_id.to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute("DELETE FROM iptv_profiles WHERE id = ?1", [&id])
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Atomically replaces all channels for `profile_id` inside one SQLite transaction,
    /// preserving any existing `is_favorite` selections matched by `stream_id`.
    pub async fn replace_iptv_channels(
        &self,
        profile_id: &str,
        channels: Vec<NewIptvChannel>,
    ) -> VesselResult<usize> {
        let pid = profile_id.to_owned();
        self.actor()
            .execute(move |db| {
                let conn = db.conn();
                conn.execute_batch("BEGIN IMMEDIATE").map_err(map_sql_err)?;

                let res = (|| -> VesselResult<usize> {
                    let mut favorites = HashSet::new();
                    {
                        let mut fav_stmt = conn
                            .prepare(
                                "SELECT stream_id FROM iptv_channels WHERE profile_id = ?1 AND is_favorite = 1",
                            )
                            .map_err(map_sql_err)?;
                        let fav_rows = fav_stmt
                            .query_map([&pid], |row| row.get::<_, String>(0))
                            .map_err(map_sql_err)?;
                        for row in fav_rows {
                            favorites.insert(row.map_err(map_sql_err)?);
                        }
                    }

                    conn.execute("DELETE FROM iptv_channels WHERE profile_id = ?1", [&pid])
                        .map_err(map_sql_err)?;

                    let count = channels.len();
                    {
                        let mut insert = conn
                            .prepare(
                                "INSERT INTO iptv_channels (
                                     profile_id, stream_id, name, group_title, logo_url, stream_url, tvg_id, is_favorite
                                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                            )
                            .map_err(map_sql_err)?;
                        for ch in channels {
                            let is_fav = i32::from(favorites.contains(&ch.stream_id));
                            insert
                                .execute((
                                    &pid,
                                    &ch.stream_id,
                                    &ch.name,
                                    &ch.group_title,
                                    &ch.logo_url,
                                    &ch.stream_url,
                                    &ch.tvg_id,
                                    is_fav,
                                ))
                                .map_err(map_sql_err)?;
                        }
                    }

                    conn.execute(
                        "UPDATE iptv_profiles
                         SET last_synced_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                         WHERE id = ?1",
                        [&pid],
                    )
                    .map_err(map_sql_err)?;

                    Ok(count)
                })();

                match res {
                    Ok(count) => {
                        conn.execute_batch("COMMIT").map_err(map_sql_err)?;
                        Ok(count)
                    }
                    Err(err) => {
                        let _ = conn.execute_batch("ROLLBACK");
                        Err(err)
                    }
                }
            })
            .await
    }

    /// Searches IPTV channels using `iptv_channels_fts` when `query` is present.
    pub async fn search_iptv_channels(
        &self,
        profile_id: Option<&str>,
        query: Option<&str>,
        group_title: Option<&str>,
        favorites_only: bool,
        limit: usize,
    ) -> VesselResult<Vec<StoredIptvChannel>> {
        let pid = profile_id.map(str::to_owned);
        let fts_expr = query.and_then(build_fts_query);
        let group = group_title.map(str::to_owned);
        let fav_flag = i32::from(favorites_only);
        let cap = i64::try_from(limit.clamp(1, 500)).unwrap_or(500);

        self.actor()
            .execute(move |db| {
                let mut rows_out = Vec::new();
                if let Some(ref fts) = fts_expr {
                    let mut stmt = db
                        .conn()
                        .prepare(
                            "SELECT c.id, c.profile_id, c.stream_id, c.name, c.group_title,
                                    c.logo_url, c.stream_url, c.tvg_id, c.is_favorite
                             FROM iptv_channels_fts f
                             JOIN iptv_channels c ON c.id = f.rowid
                             WHERE iptv_channels_fts MATCH ?1
                               AND (?2 IS NULL OR c.profile_id = ?2)
                               AND (?3 IS NULL OR c.group_title = ?3)
                               AND (?4 = 0 OR c.is_favorite = 1)
                             ORDER BY bm25(iptv_channels_fts), c.name ASC
                             LIMIT ?5",
                        )
                        .map_err(map_sql_err)?;
                    let mapped = stmt
                        .query_map((fts, &pid, &group, fav_flag, cap), channel_from_row)
                        .map_err(map_sql_err)?;
                    for r in mapped {
                        rows_out.push(r.map_err(map_sql_err)?);
                    }
                } else {
                    let mut stmt = db
                        .conn()
                        .prepare(
                            "SELECT id, profile_id, stream_id, name, group_title,
                                    logo_url, stream_url, tvg_id, is_favorite
                             FROM iptv_channels
                             WHERE (?1 IS NULL OR profile_id = ?1)
                               AND (?2 IS NULL OR group_title = ?2)
                               AND (?3 = 0 OR is_favorite = 1)
                             ORDER BY group_title ASC, name ASC
                             LIMIT ?4",
                        )
                        .map_err(map_sql_err)?;
                    let mapped = stmt
                        .query_map((&pid, &group, fav_flag, cap), channel_from_row)
                        .map_err(map_sql_err)?;
                    for r in mapped {
                        rows_out.push(r.map_err(map_sql_err)?);
                    }
                }
                Ok(rows_out)
            })
            .await
    }

    /// Lists distinct `group_title` categories for a profile's channel sidebar.
    pub async fn list_iptv_groups(&self, profile_id: &str) -> VesselResult<Vec<String>> {
        let pid = profile_id.to_owned();
        self.actor()
            .execute(move |db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT DISTINCT group_title FROM iptv_channels
                         WHERE profile_id = ?1
                         ORDER BY group_title ASC",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([&pid], |row| row.get(0))
                    .map_err(map_sql_err)?;
                let mut groups = Vec::new();
                for item in rows {
                    groups.push(item.map_err(map_sql_err)?);
                }
                Ok(groups)
            })
            .await
    }

    /// Sets or clears the `is_favorite` flag on a channel by row ID.
    pub async fn set_iptv_favorite(&self, channel_id: i64, is_favorite: bool) -> VesselResult<()> {
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "UPDATE iptv_channels SET is_favorite = ?1 WHERE id = ?2",
                        (i32::from(is_favorite), channel_id),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Replaces EPG programs for `profile_id` inside one transaction.
    pub async fn replace_epg_programs(
        &self,
        profile_id: &str,
        programs: Vec<StoredEpgProgram>,
    ) -> VesselResult<usize> {
        let pid = profile_id.to_owned();
        self.actor()
            .execute(move |db| {
                let conn = db.conn();
                conn.execute_batch("BEGIN IMMEDIATE").map_err(map_sql_err)?;
                let res = (|| -> VesselResult<usize> {
                    conn.execute("DELETE FROM epg_programs WHERE profile_id = ?1", [&pid])
                        .map_err(map_sql_err)?;
                    let count = programs.len();
                    {
                        let mut insert = conn
                            .prepare(
                                "INSERT INTO epg_programs (
                                     profile_id, channel_tvg_id, title, description, start_ts, end_ts
                                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            )
                            .map_err(map_sql_err)?;
                        for prog in programs {
                            insert
                                .execute((
                                    &pid,
                                    &prog.channel_tvg_id,
                                    &prog.title,
                                    &prog.description,
                                    prog.start_ts,
                                    prog.end_ts,
                                ))
                                .map_err(map_sql_err)?;
                        }
                    }
                    Ok(count)
                })();

                match res {
                    Ok(count) => {
                        conn.execute_batch("COMMIT").map_err(map_sql_err)?;
                        Ok(count)
                    }
                    Err(err) => {
                        let _ = conn.execute_batch("ROLLBACK");
                        Err(err)
                    }
                }
            })
            .await
    }

    /// Deletes expired EPG entries whose `end_ts <= before_ts`.
    pub async fn prune_expired_epg(&self, before_ts: i64) -> VesselResult<usize> {
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute("DELETE FROM epg_programs WHERE end_ts <= ?1", [before_ts])
                    .map_err(map_sql_err)
            })
            .await
    }

    /// Queries Now & Next EPG entries for `(profile_id, channel_tvg_id)` at Unix `now_ts`.
    pub async fn get_epg_now_next(
        &self,
        profile_id: &str,
        channel_tvg_id: &str,
        now_ts: i64,
    ) -> VesselResult<StoredEpgNowNext> {
        let pid = profile_id.to_owned();
        let tvg = channel_tvg_id.to_owned();
        self.actor()
            .execute(move |db| {
                let now = db
                    .conn()
                    .query_row(
                        "SELECT channel_tvg_id, title, description, start_ts, end_ts
                         FROM epg_programs
                         WHERE profile_id = ?1 AND channel_tvg_id = ?2
                           AND start_ts <= ?3 AND end_ts > ?3
                         ORDER BY start_ts DESC LIMIT 1",
                        (&pid, &tvg, now_ts),
                        |row| {
                            Ok(StoredEpgProgram {
                                channel_tvg_id: row.get(0)?,
                                title: row.get(1)?,
                                description: row.get(2)?,
                                start_ts: row.get(3)?,
                                end_ts: row.get(4)?,
                            })
                        },
                    )
                    .optional()
                    .map_err(map_sql_err)?;

                let next_after = now.as_ref().map_or(now_ts, |prog| prog.end_ts);
                let next = db
                    .conn()
                    .query_row(
                        "SELECT channel_tvg_id, title, description, start_ts, end_ts
                         FROM epg_programs
                         WHERE profile_id = ?1 AND channel_tvg_id = ?2
                           AND start_ts >= ?3
                         ORDER BY start_ts ASC LIMIT 1",
                        (&pid, &tvg, next_after),
                        |row| {
                            Ok(StoredEpgProgram {
                                channel_tvg_id: row.get(0)?,
                                title: row.get(1)?,
                                description: row.get(2)?,
                                start_ts: row.get(3)?,
                                end_ts: row.get(4)?,
                            })
                        },
                    )
                    .optional()
                    .map_err(map_sql_err)?;

                Ok(StoredEpgNowNext { now, next })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        NewIptvChannel, SqliteStorage, StorageActor, StoredEpgProgram, StoredIptvKind,
        StoredIptvProfile,
    };

    #[tokio::test]
    async fn iptv_fts5_search_preserves_favorites_across_sync_and_computes_epg_now_next() {
        let actor = StorageActor::open_in_memory()
            .await
            .expect("open in memory");
        let storage = SqliteStorage::new(actor);

        let profile = StoredIptvProfile {
            id: "iptv-anthology".into(),
            name: "Anthology TV".into(),
            kind: StoredIptvKind::M3u,
            server_url: "https://iptv.anthology.example/list.m3u".into(),
            username: None,
            secret_ref: None,
            epg_url: Some("https://iptv.anthology.example/epg.xml".into()),
            user_agent: Some("VesselPlayer/1.0".into()),
            last_synced_at: None,
        };
        storage.upsert_iptv_profile(&profile).await.expect("upsert");

        storage
            .replace_iptv_channels(
                &profile.id,
                vec![
                    NewIptvChannel {
                        stream_id: "ch-news".into(),
                        name: "Anthology World News".into(),
                        group_title: "News".into(),
                        logo_url: None,
                        stream_url: "https://iptv.anthology.example/news.m3u8".into(),
                        tvg_id: Some("news.world".into()),
                    },
                    NewIptvChannel {
                        stream_id: "ch-sport".into(),
                        name: "Galaxy Sports Arena".into(),
                        group_title: "Sports".into(),
                        logo_url: None,
                        stream_url: "https://iptv.anthology.example/sport.ts".into(),
                        tvg_id: Some("sport.1".into()),
                    },
                ],
            )
            .await
            .expect("insert channels");

        let found = storage
            .search_iptv_channels(Some(&profile.id), Some("world"), None, false, 20)
            .await
            .expect("fts5 search");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].stream_id, "ch-news");

        storage
            .set_iptv_favorite(found[0].id, true)
            .await
            .expect("mark favorite");

        // Re-sync channels and verify `is_favorite` survives by `(profile_id, stream_id)`.
        storage
            .replace_iptv_channels(
                &profile.id,
                vec![NewIptvChannel {
                    stream_id: "ch-news".into(),
                    name: "Anthology World News 4K".into(),
                    group_title: "News".into(),
                    logo_url: None,
                    stream_url: "https://iptv.anthology.example/news4k.m3u8".into(),
                    tvg_id: Some("news.world".into()),
                }],
            )
            .await
            .expect("resync channels");

        let favs = storage
            .search_iptv_channels(Some(&profile.id), None, None, true, 20)
            .await
            .expect("list favs");
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].name, "Anthology World News 4K");
        assert!(favs[0].is_favorite);

        storage
            .replace_epg_programs(
                &profile.id,
                vec![
                    StoredEpgProgram {
                        channel_tvg_id: "news.world".into(),
                        title: "Expired".into(),
                        description: None,
                        start_ts: 100,
                        end_ts: 200,
                    },
                    StoredEpgProgram {
                        channel_tvg_id: "news.world".into(),
                        title: "Current Show".into(),
                        description: Some("Live".into()),
                        start_ts: 200,
                        end_ts: 400,
                    },
                    StoredEpgProgram {
                        channel_tvg_id: "news.world".into(),
                        title: "Upcoming Show".into(),
                        description: None,
                        start_ts: 400,
                        end_ts: 600,
                    },
                ],
            )
            .await
            .expect("insert epg");

        let pruned = storage.prune_expired_epg(200).await.expect("prune");
        assert_eq!(pruned, 1);

        let nn = storage
            .get_epg_now_next(&profile.id, "news.world", 250)
            .await
            .expect("now next");
        assert_eq!(
            nn.now.as_ref().map(|p| p.title.as_str()),
            Some("Current Show")
        );
        assert_eq!(
            nn.next.as_ref().map(|p| p.title.as_str()),
            Some("Upcoming Show")
        );
    }
}
