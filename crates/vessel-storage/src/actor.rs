//! Single-writer SQLite actor powered by `tokio::sync::mpsc` and `oneshot` channels.

use std::path::PathBuf;

use tokio::sync::{mpsc, oneshot};
use vessel_domain::{VesselError, VesselPaths, VesselResult};

use crate::StorageDb;

const ACTOR_QUEUE_CAPACITY: usize = 128;

type DbJob = Box<dyn FnOnce(&mut StorageDb) + Send>;

/// Cloneable async handle communicating with a dedicated single-thread SQLite actor.
#[derive(Debug, Clone)]
pub struct StorageActor {
    tx: mpsc::Sender<DbJob>,
}

impl StorageActor {
    /// Spawns the SQLite actor thread and opens `<VesselRoot>` database storage.
    pub async fn open_with_paths(paths: &VesselPaths) -> VesselResult<Self> {
        let paths = paths.clone();
        Self::spawn_with(move || StorageDb::open_with_paths(&paths)).await
    }

    /// Spawns the SQLite actor thread for a database file path.
    pub async fn open(path: impl Into<PathBuf>) -> VesselResult<Self> {
        let path: PathBuf = path.into();
        Self::spawn_with(move || StorageDb::open(&path)).await
    }

    /// Spawns the SQLite actor thread backed by an isolated in-memory database.
    pub async fn open_in_memory() -> VesselResult<Self> {
        Self::spawn_with(StorageDb::open_in_memory).await
    }

    /// Spawns a dedicated OS thread that owns the `StorageDb` connection and serves `DbJob` requests.
    async fn spawn_with<F>(opener: F) -> VesselResult<Self>
    where
        F: FnOnce() -> VesselResult<StorageDb> + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<DbJob>(ACTOR_QUEUE_CAPACITY);
        let (init_tx, init_rx) = oneshot::channel::<VesselResult<()>>();

        std::thread::Builder::new()
            .name("vessel-sqlite-actor".into())
            .spawn(move || {
                let mut db = match opener() {
                    Ok(db) => {
                        let _ = init_tx.send(Ok(()));
                        db
                    }
                    Err(err) => {
                        let _ = init_tx.send(Err(err));
                        return;
                    }
                };

                while let Some(job) = rx.blocking_recv() {
                    job(&mut db);
                }
            })
            .map_err(|err| VesselError::Storage(format!("spawn sqlite actor thread: {err}")))?;

        match init_rx.await {
            Ok(Ok(())) => Ok(Self { tx }),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(VesselError::Storage(
                "sqlite actor terminated during initialization".into(),
            )),
        }
    }

    /// Runs an operation exclusively on the SQLite actor thread and awaits its result.
    pub async fn execute<T, F>(&self, op: F) -> VesselResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut StorageDb) -> VesselResult<T> + Send + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel::<VesselResult<T>>();
        let job: DbJob = Box::new(move |db| {
            let _ = reply_tx.send(op(db));
        });

        self.tx
            .send(job)
            .await
            .map_err(|_| VesselError::Storage("sqlite actor channel closed".into()))?;

        reply_rx
            .await
            .map_err(|_| VesselError::Storage("sqlite actor dropped response".into()))?
    }

    /// Convenience check returning the current migration version via the actor channel.
    pub async fn schema_version(&self) -> VesselResult<i64> {
        self.execute(|db| db.schema_version()).await
    }
}

#[cfg(test)]
mod tests {
    use tokio::task::JoinSet;
    use vessel_domain::VesselError;

    use super::StorageActor;
    use crate::map_sql_err;

    #[tokio::test]
    async fn serializes_concurrent_writes_without_busy_errors() {
        let actor = StorageActor::open_in_memory()
            .await
            .expect("spawn in-memory actor");
        assert_eq!(actor.schema_version().await.expect("schema version"), 1);

        let mut tasks = JoinSet::new();
        for idx in 0..16_u32 {
            let handle = actor.clone();
            tasks.spawn(async move {
                handle
                    .execute(move |db| {
                        db.conn()
                            .execute(
                                "INSERT INTO settings (key, value_json) VALUES (?1, ?2)",
                                (format!("k_{idx}"), format!("{idx}")),
                            )
                            .map_err(map_sql_err)?;
                        Ok(())
                    })
                    .await
            });
        }

        while let Some(joined) = tasks.join_next().await {
            joined
                .expect("tokio task should not panic")
                .expect("sqlite insert should succeed");
        }

        let count: i64 = actor
            .execute(|db| {
                db.conn()
                    .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
                    .map_err(map_sql_err)
            })
            .await
            .expect("count inserted settings");
        assert_eq!(count, 16);
    }

    #[tokio::test]
    async fn surfaces_initialization_error_from_actor_thread() {
        let dir = std::env::temp_dir().join(format!("vessel-actor-err-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let file_blocker = dir.join("not_a_dir");
        std::fs::write(&file_blocker, b"x").expect("create blocking file");

        let res = StorageActor::open(file_blocker.join("vessel.db")).await;
        let _ = std::fs::remove_dir_all(&dir);

        assert!(matches!(res, Err(VesselError::Storage(_))));
    }
}
