-- Phase 2 & 3: Sync tombstones (library_items, watch_progress, metadata_overrides),
-- tracker account preferences, offline tracker retry queue, and cloud sync state.

ALTER TABLE library_items ADD COLUMN deleted_at TEXT;
ALTER TABLE library_items ADD COLUMN origin_device_id TEXT;
CREATE INDEX IF NOT EXISTS idx_library_sync ON library_items(updated_at, deleted_at);

ALTER TABLE watch_progress ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
ALTER TABLE watch_progress ADD COLUMN deleted_at TEXT;
ALTER TABLE watch_progress ADD COLUMN origin_device_id TEXT;
UPDATE watch_progress
SET updated_at = CASE
    WHEN trim(last_watched_at) != '' THEN last_watched_at
    ELSE strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
END
WHERE updated_at = '';
CREATE INDEX IF NOT EXISTS idx_progress_sync ON watch_progress(updated_at, deleted_at);

ALTER TABLE metadata_overrides ADD COLUMN deleted_at TEXT;
ALTER TABLE metadata_overrides ADD COLUMN origin_device_id TEXT;

CREATE TABLE IF NOT EXISTS tracker_accounts (
    provider TEXT PRIMARY KEY,
    username TEXT,
    user_id TEXT,
    auto_scrobble INTEGER NOT NULL DEFAULT 1,
    sync_library INTEGER NOT NULL DEFAULT 1,
    last_synced_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracker_sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    next_retry_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tracker_queue_due ON tracker_sync_queue(provider, next_retry_at);

CREATE TABLE IF NOT EXISTS sync_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
