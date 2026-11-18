CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS repositories (
    id TEXT PRIMARY KEY,
    source_input TEXT NOT NULL UNIQUE,
    resolved_url TEXT NOT NULL,
    name TEXT NOT NULL,
    last_etag TEXT,
    last_commit_sha TEXT,
    is_official INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS plugins (
    id TEXT PRIMARY KEY,
    repo_id TEXT REFERENCES repositories(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    capabilities_json TEXT NOT NULL,
    languages_json TEXT NOT NULL,
    source_type TEXT NOT NULL,
    local_path TEXT NOT NULL,
    sha256 TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    allow_dynamic_domains INTEGER NOT NULL DEFAULT 0,
    manifest_json TEXT NOT NULL,
    installed_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS plugin_allowed_domains (
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    domain TEXT NOT NULL,
    grant_source TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (plugin_id, domain)
);

CREATE TABLE IF NOT EXISTS plugin_kv (
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (plugin_id, key)
);

CREATE TABLE IF NOT EXISTS secrets (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS library_items (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    native_id TEXT NOT NULL,
    canonical_id TEXT,
    media_type TEXT NOT NULL,
    is_anime INTEGER NOT NULL DEFAULT 0,
    title TEXT NOT NULL,
    poster_url TEXT,
    year INTEGER,
    status TEXT NOT NULL,
    added_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (plugin_id, native_id)
);
CREATE INDEX IF NOT EXISTS idx_library_canonical ON library_items(canonical_id);

CREATE TABLE IF NOT EXISTS watch_progress (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    native_id TEXT NOT NULL,
    stream_id TEXT NOT NULL,
    canonical_id TEXT,
    season INTEGER NOT NULL DEFAULT 1,
    episode INTEGER NOT NULL DEFAULT 1,
    absolute_episode INTEGER,
    position_ms INTEGER NOT NULL DEFAULT 0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0,
    audio_lang TEXT,
    subtitle_lang TEXT,
    subtitle_delay_ms INTEGER NOT NULL DEFAULT 0,
    last_watched_at TEXT NOT NULL,
    UNIQUE (plugin_id, native_id, season, episode)
);
CREATE INDEX IF NOT EXISTS idx_progress_canonical
    ON watch_progress(canonical_id, absolute_episode, last_watched_at DESC);

CREATE TABLE IF NOT EXISTS metadata_cache (
    cache_key TEXT PRIMARY KEY,
    payload_json TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS metadata_overrides (
    plugin_id TEXT NOT NULL,
    native_id TEXT NOT NULL,
    canonical_id TEXT,
    is_disabled INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (plugin_id, native_id)
);

CREATE TABLE IF NOT EXISTS iptv_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    server_url TEXT NOT NULL,
    username TEXT,
    secret_ref TEXT,
    epg_url TEXT,
    user_agent TEXT,
    last_synced_at TEXT
);

CREATE TABLE IF NOT EXISTS iptv_channels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL REFERENCES iptv_profiles(id) ON DELETE CASCADE,
    stream_id TEXT NOT NULL,
    name TEXT NOT NULL,
    group_title TEXT NOT NULL DEFAULT '',
    logo_url TEXT,
    stream_url TEXT NOT NULL,
    tvg_id TEXT,
    is_favorite INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_iptv_profile_group ON iptv_channels(profile_id, group_title);

CREATE VIRTUAL TABLE IF NOT EXISTS iptv_channels_fts
    USING fts5(name, group_title, content='iptv_channels', content_rowid='id');

CREATE TRIGGER IF NOT EXISTS iptv_channels_ai AFTER INSERT ON iptv_channels BEGIN
    INSERT INTO iptv_channels_fts(rowid, name, group_title)
    VALUES (new.id, new.name, new.group_title);
END;

CREATE TRIGGER IF NOT EXISTS iptv_channels_ad AFTER DELETE ON iptv_channels BEGIN
    INSERT INTO iptv_channels_fts(iptv_channels_fts, rowid, name, group_title)
    VALUES ('delete', old.id, old.name, old.group_title);
END;

CREATE TRIGGER IF NOT EXISTS iptv_channels_au AFTER UPDATE ON iptv_channels BEGIN
    INSERT INTO iptv_channels_fts(iptv_channels_fts, rowid, name, group_title)
    VALUES ('delete', old.id, old.name, old.group_title);
    INSERT INTO iptv_channels_fts(rowid, name, group_title)
    VALUES (new.id, new.name, new.group_title);
END;

CREATE TABLE IF NOT EXISTS epg_programs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL REFERENCES iptv_profiles(id) ON DELETE CASCADE,
    channel_tvg_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    start_ts INTEGER NOT NULL,
    end_ts INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_epg_lookup
    ON epg_programs(profile_id, channel_tvg_id, start_ts, end_ts);

CREATE TABLE IF NOT EXISTS downloads (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    native_id TEXT NOT NULL,
    title TEXT NOT NULL,
    season INTEGER,
    episode INTEGER,
    kind TEXT NOT NULL,
    source_url TEXT NOT NULL,
    headers_json TEXT NOT NULL,
    local_dir TEXT NOT NULL,
    local_media_path TEXT,
    status TEXT NOT NULL,
    bytes_downloaded INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER,
    segments_done INTEGER NOT NULL DEFAULT 0,
    segments_total INTEGER NOT NULL DEFAULT 0,
    subtitles_json TEXT NOT NULL DEFAULT '[]',
    error_msg TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS download_segments (
    download_id TEXT NOT NULL REFERENCES downloads(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    url TEXT NOT NULL,
    file_name TEXT NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (download_id, seq)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ts_compile_cache (
    source_hash TEXT PRIMARY KEY,
    compiled_js TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
