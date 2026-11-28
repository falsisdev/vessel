//! Persistent tracker account preferences and offline retry queue (`tracker_accounts`, `tracker_sync_queue`).

use std::collections::BTreeMap;

use rusqlite::OptionalExtension;
use vessel_domain::{VesselError, VesselResult};

use crate::{SqliteStorage, map_sql_err};

/// Persisted tracker connection metadata & sync preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTrackerAccount {
    pub provider: String,
    pub username: Option<String>,
    pub user_id: Option<String>,
    pub auto_scrobble: bool,
    pub sync_library: bool,
    pub last_synced_at: Option<String>,
    pub updated_at: String,
}

/// Outbound scrobble/watchlist action persisted while offline or rate-limited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedTrackerAction {
    pub id: i64,
    pub provider: String,
    pub payload_json: String,
    pub attempts: u32,
    pub last_error: Option<String>,
    pub next_retry_at: String,
    pub created_at: String,
}

impl SqliteStorage {
    /// Lists all saved tracker account rows ordered by provider name.
    pub async fn list_tracker_accounts(&self) -> VesselResult<Vec<StoredTrackerAccount>> {
        self.actor()
            .execute(|db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT provider, username, user_id, auto_scrobble, sync_library, last_synced_at, updated_at
                         FROM tracker_accounts
                         ORDER BY provider ASC",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok(StoredTrackerAccount {
                            provider: row.get(0)?,
                            username: row.get(1)?,
                            user_id: row.get(2)?,
                            auto_scrobble: row.get::<_, i64>(3)? != 0,
                            sync_library: row.get::<_, i64>(4)? != 0,
                            last_synced_at: row.get(5)?,
                            updated_at: row.get(6)?,
                        })
                    })
                    .map_err(map_sql_err)?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r.map_err(map_sql_err)?);
                }
                Ok(out)
            })
            .await
    }

    /// Reads one tracker account row by canonical `provider` key.
    pub async fn get_tracker_account(
        &self,
        provider: &str,
    ) -> VesselResult<Option<StoredTrackerAccount>> {
        let provider = provider.trim().to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .query_row(
                        "SELECT provider, username, user_id, auto_scrobble, sync_library, last_synced_at, updated_at
                         FROM tracker_accounts WHERE provider = ?1",
                        [&provider],
                        |row| {
                            Ok(StoredTrackerAccount {
                                provider: row.get(0)?,
                                username: row.get(1)?,
                                user_id: row.get(2)?,
                                auto_scrobble: row.get::<_, i64>(3)? != 0,
                                sync_library: row.get::<_, i64>(4)? != 0,
                                last_synced_at: row.get(5)?,
                                updated_at: row.get(6)?,
                            })
                        },
                    )
                    .optional()
                    .map_err(map_sql_err)
            })
            .await
    }

    /// Inserts or updates a tracker account profile and preferences.
    pub async fn upsert_tracker_account(&self, account: &StoredTrackerAccount) -> VesselResult<()> {
        let account = account.clone();
        if account.provider.trim().is_empty() {
            return Err(VesselError::InvalidInput("tracker provider cannot be empty".into()));
        }
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO tracker_accounts (
                             provider, username, user_id, auto_scrobble, sync_library, last_synced_at, updated_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                         ON CONFLICT(provider) DO UPDATE SET
                             username = COALESCE(excluded.username, tracker_accounts.username),
                             user_id = COALESCE(excluded.user_id, tracker_accounts.user_id),
                             auto_scrobble = excluded.auto_scrobble,
                             sync_library = excluded.sync_library,
                             last_synced_at = COALESCE(excluded.last_synced_at, tracker_accounts.last_synced_at),
                             updated_at = excluded.updated_at",
                        (
                            &account.provider,
                            &account.username,
                            &account.user_id,
                            i32::from(account.auto_scrobble),
                            i32::from(account.sync_library),
                            &account.last_synced_at,
                        ),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Deletes stored tracker metadata and clears queued sync actions for `provider`.
    pub async fn delete_tracker_account(&self, provider: &str) -> VesselResult<()> {
        let provider = provider.trim().to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute("DELETE FROM tracker_accounts WHERE provider = ?1", [&provider])
                    .map_err(map_sql_err)?;
                db.conn()
                    .execute("DELETE FROM tracker_sync_queue WHERE provider = ?1", [&provider])
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Enqueues a failed/offline tracker sync payload for retry with exponential backoff.
    pub async fn enqueue_tracker_action(
        &self,
        provider: &str,
        payload_json: &str,
        last_error: Option<&str>,
    ) -> VesselResult<i64> {
        let provider = provider.trim().to_owned();
        let payload_json = payload_json.to_owned();
        let last_error = last_error.map(str::to_owned);
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO tracker_sync_queue (
                             provider, payload_json, attempts, last_error, next_retry_at, created_at
                         ) VALUES (
                             ?1, ?2, 1, ?3,
                             strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '+30 seconds'),
                             strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                         )",
                        (&provider, &payload_json, &last_error),
                    )
                    .map_err(map_sql_err)?;
                Ok(db.conn().last_insert_rowid())
            })
            .await
    }

    /// Lists queued tracker actions whose `next_retry_at <= now` (and `attempts < 6`).
    pub async fn list_due_tracker_actions(
        &self,
        limit: usize,
    ) -> VesselResult<Vec<QueuedTrackerAction>> {
        let cap = limit.clamp(1, 200) as i64;
        self.actor()
            .execute(move |db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT id, provider, payload_json, attempts, last_error, next_retry_at, created_at
                         FROM tracker_sync_queue
                         WHERE attempts < 6
                           AND next_retry_at <= strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                         ORDER BY id ASC
                         LIMIT ?1",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([cap], |row| {
                        Ok(QueuedTrackerAction {
                            id: row.get(0)?,
                            provider: row.get(1)?,
                            payload_json: row.get(2)?,
                            attempts: row.get::<_, u32>(3)?,
                            last_error: row.get(4)?,
                            next_retry_at: row.get(5)?,
                            created_at: row.get(6)?,
                        })
                    })
                    .map_err(map_sql_err)?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r.map_err(map_sql_err)?);
                }
                Ok(out)
            })
            .await
    }

    /// Counts pending queued sync actions grouped by `provider`.
    pub async fn count_pending_tracker_actions(&self) -> VesselResult<BTreeMap<String, usize>> {
        self.actor()
            .execute(|db| {
                let mut stmt = db
                    .conn()
                    .prepare(
                        "SELECT provider, COUNT(*) FROM tracker_sync_queue
                         WHERE attempts < 6
                         GROUP BY provider",
                    )
                    .map_err(map_sql_err)?;
                let rows = stmt
                    .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
                    .map_err(map_sql_err)?;
                let mut map = BTreeMap::new();
                for r in rows {
                    let (p, c) = r.map_err(map_sql_err)?;
                    map.insert(p, c.max(0) as usize);
                }
                Ok(map)
            })
            .await
    }

    /// Updates `last_synced_at` to the current UTC ISO-8601 timestamp for `provider`.
    pub async fn touch_tracker_last_synced(&self, provider: &str) -> VesselResult<()> {
        let provider = provider.trim().to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "INSERT INTO tracker_accounts (provider, last_synced_at, updated_at)
                         VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                         ON CONFLICT(provider) DO UPDATE SET
                             last_synced_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
                        [&provider],
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Removes a successfully dispatched action from `tracker_sync_queue`.
    pub async fn delete_queued_tracker_action(&self, id: i64) -> VesselResult<()> {
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute("DELETE FROM tracker_sync_queue WHERE id = ?1", [id])
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }

    /// Records a retry failure with bounded exponential backoff (`30s .. 3600s`).
    pub async fn mark_tracker_action_failed(
        &self,
        id: i64,
        attempts: u32,
        error: &str,
    ) -> VesselResult<()> {
        let next_attempts = attempts.saturating_add(1);
        let backoff_secs = 30u64.saturating_mul(1u64 << attempts.min(7)).min(3600);
        let delay_mod = format!("+{backoff_secs} seconds");
        let error = error.to_owned();
        self.actor()
            .execute(move |db| {
                db.conn()
                    .execute(
                        "UPDATE tracker_sync_queue
                         SET attempts = ?1,
                             last_error = ?2,
                             next_retry_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', ?3)
                         WHERE id = ?4",
                        (next_attempts, &error, &delay_mod, id),
                    )
                    .map_err(map_sql_err)?;
                Ok(())
            })
            .await
    }
}
