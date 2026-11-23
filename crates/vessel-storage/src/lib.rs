//! Local SQLite persistence (`bundled` + WAL + FTS5) for Vessel.

pub mod actor;
pub mod backup;
pub mod library;
pub mod metadata;
pub mod plugins;
pub mod secret;
pub mod store;

use std::path::Path;

use rusqlite::Connection;
use vessel_domain::{VesselError, VesselPaths, VesselResult};

pub use actor::StorageActor;
pub use backup::{BACKUP_SCHEMA_VERSION, BackupImportMode, BackupPluginEntry, VesselBackup};
pub use metadata::MetadataOverride;
pub use plugins::StoredPlugin;
pub use secret::SqliteSecretStore;
pub use store::{SqliteStorage, UI_PREFERENCES_KEY};

const MIGRATION_001_INIT: &str = include_str!("../migrations/001_init.sql");

/// Versioned SQL migrations embedded at compile time.
const MIGRATIONS: &[(i64, &str)] = &[(1, MIGRATION_001_INIT)];

/// Converts a SQLite driver error into a domain `VesselError::Storage`.
///
/// Defined as a helper function rather than `impl From<rusqlite::Error> for VesselError`
/// because Rust's orphan rule forbids implementing a foreign trait (`From`) for a
/// type defined in another crate (`VesselError` lives in `vessel-domain`).
pub(crate) fn map_sql_err(err: rusqlite::Error) -> VesselError {
    VesselError::Storage(err.to_string())
}

/// Manages a configured SQLite connection with schema migrations applied.
#[derive(Debug)]
pub struct StorageDb {
    conn: Connection,
}

impl StorageDb {
    /// Provisions `<VesselRoot>` directories, opens `vessel.db`, configures WAL/FKs, and runs migrations.
    pub fn open_with_paths(paths: &VesselPaths) -> VesselResult<Self> {
        for dir in paths.required_dirs() {
            std::fs::create_dir_all(&dir)
                .map_err(|err| VesselError::Storage(format!("create {}: {err}", dir.display())))?;
        }
        Self::open(&paths.database_file())
    }

    /// Opens (or creates) a SQLite database file, configures pragmas, and applies pending migrations.
    pub fn open(path: &Path) -> VesselResult<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|err| {
                VesselError::Storage(format!("create {}: {err}", parent.display()))
            })?;
        }
        let conn = Connection::open(path).map_err(map_sql_err)?;
        Self::from_connection(conn)
    }

    /// Opens an isolated in-memory SQLite database with all migrations applied (useful for tests).
    pub fn open_in_memory() -> VesselResult<Self> {
        let conn = Connection::open_in_memory().map_err(map_sql_err)?;
        Self::from_connection(conn)
    }

    fn from_connection(mut conn: Connection) -> VesselResult<Self> {
        configure_connection(&conn)?;
        run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// Borrows the underlying SQLite connection for crate-internal repositories.
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Mutably borrows the underlying SQLite connection for transactional operations.
    pub(crate) fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Returns the highest applied migration version, or `0` if none have run.
    pub fn schema_version(&self) -> VesselResult<i64> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(map_sql_err)
    }
}

fn configure_connection(conn: &Connection) -> VesselResult<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )
    .map_err(map_sql_err)
}

fn run_migrations(conn: &mut Connection) -> VesselResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL
         );",
    )
    .map_err(map_sql_err)?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(map_sql_err)?;

    for &(version, sql) in MIGRATIONS {
        if version <= current_version {
            continue;
        }
        let tx = conn.transaction().map_err(map_sql_err)?;
        tx.execute_batch(sql).map_err(map_sql_err)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            [version],
        )
        .map_err(map_sql_err)?;
        tx.commit().map_err(map_sql_err)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use vessel_domain::VesselPaths;

    use super::StorageDb;

    struct TempDirGuard(PathBuf);

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock after epoch")
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn applies_initial_migration_and_supports_fts5_triggers() {
        let db = StorageDb::open_in_memory().expect("open in-memory storage");
        assert_eq!(db.schema_version().expect("read schema version"), 1);

        let conn = &db.conn;
        conn.execute(
            "INSERT INTO iptv_profiles (id, name, kind, server_url) VALUES ('p1', 'Home', 'm3u', 'https://example.com/list.m3u')",
            [],
        )
        .expect("insert iptv profile");

        conn.execute(
            "INSERT INTO iptv_channels (profile_id, stream_id, name, group_title, stream_url)
             VALUES ('p1', 'ch1', 'TRT Belgesel HD', 'Documentary', 'https://example.com/trt.ts')",
            [],
        )
        .expect("insert iptv channel");

        let matched: String = conn
            .query_row(
                "SELECT c.name FROM iptv_channels_fts f
                 JOIN iptv_channels c ON c.id = f.rowid
                 WHERE iptv_channels_fts MATCH 'Belgesel'",
                [],
                |row| row.get(0),
            )
            .expect("query fts5");
        assert_eq!(matched, "TRT Belgesel HD");

        conn.execute("DELETE FROM iptv_channels WHERE stream_id = 'ch1'", [])
            .expect("delete iptv channel");

        let fts_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM iptv_channels_fts WHERE iptv_channels_fts MATCH 'Belgesel'",
                [],
                |row| row.get(0),
            )
            .expect("count fts5 after delete");
        assert_eq!(fts_count, 0);
    }

    #[test]
    fn provisions_vessel_root_with_wal_foreign_keys_and_idempotent_reopen() {
        let temp = TempDirGuard::new("vessel-storage-test");
        let paths = VesselPaths::new(&temp.0).expect("valid VesselPaths");

        let db = StorageDb::open_with_paths(&paths).expect("open StorageDb");
        assert_eq!(db.schema_version().expect("version"), 1);
        for dir in paths.required_dirs() {
            assert!(dir.is_dir(), "expected dir {} to exist", dir.display());
        }

        let journal_mode: String = db
            .conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .expect("read journal_mode");
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let fk_enabled: i64 = db
            .conn
            .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
            .expect("read foreign_keys");
        assert_eq!(fk_enabled, 1);

        drop(db);

        let reopened = StorageDb::open_with_paths(&paths).expect("reopen StorageDb idempotently");
        assert_eq!(reopened.schema_version().expect("version after reopen"), 1);
    }
}
