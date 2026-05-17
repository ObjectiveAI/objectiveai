//! SQLite-backed persistent cache client.

use objectiveai_sdk::error::ResponseError;
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct SqlitePersistentCacheClient {
    conn: Mutex<Connection>,
    /// Tracks which namespace tables have been ensured to exist.
    ensured_tables: Mutex<HashSet<&'static str>>,
    /// How long transient (`permanent = false`) rows are kept before
    /// they're pruned on the next query.
    transient_ttl_ms: u64,
}

impl SqlitePersistentCacheClient {
    pub fn new(config_base_dir: PathBuf, transient_ttl: Duration) -> Result<Self, rusqlite::Error> {
        let db_path = config_base_dir.join("cache.sqlite");

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        Ok(Self {
            conn: Mutex::new(conn),
            ensured_tables: Mutex::new(HashSet::new()),
            transient_ttl_ms: transient_ttl.as_millis().min(u64::MAX as u128) as u64,
        })
    }

    /// Ensures the table for the given namespace exists.
    fn ensure_table(&self, conn: &Connection, namespace: &'static str) -> Result<(), rusqlite::Error> {
        {
            let tables = self.ensured_tables.lock().unwrap();
            if tables.contains(namespace) {
                return Ok(());
            }
        }

        // Namespace is &'static str from our own code, safe to interpolate.
        conn.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS [{namespace}] (\
                key TEXT PRIMARY KEY NOT NULL, \
                value TEXT NOT NULL, \
                permanent BOOLEAN NOT NULL DEFAULT 0, \
                created_at INTEGER NOT NULL DEFAULT 0\
            )"
        ))?;

        // Cache files created before `created_at` existed need the column
        // added in-place. `ALTER TABLE ... ADD COLUMN` is idempotent only
        // if we tolerate the "duplicate column" error from a second run.
        match conn.execute_batch(&format!(
            "ALTER TABLE [{namespace}] ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0"
        )) {
            Ok(()) => {}
            Err(e) if is_duplicate_column_error(&e) => {}
            Err(e) => return Err(e),
        }

        let mut tables = self.ensured_tables.lock().unwrap();
        tables.insert(namespace);
        Ok(())
    }

    /// Deletes transient rows whose `created_at` is older than the TTL.
    fn prune_transient(&self, conn: &Connection, namespace: &'static str, now_ms: u64) -> Result<(), rusqlite::Error> {
        let cutoff = now_ms.saturating_sub(self.transient_ttl_ms);
        conn.execute(
            &format!("DELETE FROM [{namespace}] WHERE permanent = 0 AND created_at < ?1"),
            rusqlite::params![cutoff as i64],
        )?;
        Ok(())
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

fn is_duplicate_column_error(e: &rusqlite::Error) -> bool {
    match e {
        rusqlite::Error::SqliteFailure(_, Some(msg)) => msg.contains("duplicate column"),
        _ => false,
    }
}

fn sqlite_err(e: rusqlite::Error) -> ResponseError {
    ResponseError {
        code: 500,
        message: serde_json::json!({ "error": format!("persistent cache error: {e}") }),
    }
}

impl super::PersistentCacheClient for SqlitePersistentCacheClient {
    async fn get(&self, namespace: &'static str, key: &str) -> Result<Option<String>, ResponseError> {
        let conn = self.conn.lock().unwrap();
        self.ensure_table(&conn, namespace).map_err(sqlite_err)?;
        self.prune_transient(&conn, namespace, now_ms()).map_err(sqlite_err)?;

        let mut stmt = conn
            .prepare_cached(&format!("SELECT value FROM [{namespace}] WHERE key = ?1"))
            .map_err(sqlite_err)?;

        stmt.query_row(rusqlite::params![key], |row| row.get::<_, String>(0))
            .optional()
            .map_err(sqlite_err)
    }

    async fn set(&self, namespace: &'static str, key: &str, value: &str, permanent: bool) -> Result<(), ResponseError> {
        let conn = self.conn.lock().unwrap();
        self.ensure_table(&conn, namespace).map_err(sqlite_err)?;
        let now = now_ms();
        self.prune_transient(&conn, namespace, now).map_err(sqlite_err)?;

        conn.execute(
            &format!("INSERT OR REPLACE INTO [{namespace}] (key, value, permanent, created_at) VALUES (?1, ?2, ?3, ?4)"),
            rusqlite::params![key, value, permanent, now as i64],
        )
        .map_err(sqlite_err)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctx::persistent_cache::PersistentCacheClient;
    use tempfile::TempDir;

    const NS: &str = "test_ns";

    fn row_count(client: &SqlitePersistentCacheClient, namespace: &'static str) -> i64 {
        let conn = client.conn.lock().unwrap();
        conn.query_row(&format!("SELECT COUNT(*) FROM [{namespace}]"), [], |r| r.get(0))
            .unwrap()
    }

    #[tokio::test]
    async fn permanent_rows_survive_prune() {
        let dir = TempDir::new().unwrap();
        let client = SqlitePersistentCacheClient::new(dir.path().to_path_buf(), Duration::from_millis(50)).unwrap();

        client.set(NS, "k", "v", true).await.unwrap();
        tokio::time::sleep(Duration::from_millis(120)).await;

        // Trigger a query so prune runs.
        let got = client.get(NS, "k").await.unwrap();
        assert_eq!(got.as_deref(), Some("v"));
        assert_eq!(row_count(&client, NS), 1);
    }

    #[tokio::test]
    async fn transient_rows_evicted_after_ttl() {
        let dir = TempDir::new().unwrap();
        let client = SqlitePersistentCacheClient::new(dir.path().to_path_buf(), Duration::from_millis(50)).unwrap();

        client.set(NS, "k", "v", false).await.unwrap();
        tokio::time::sleep(Duration::from_millis(120)).await;

        let got = client.get(NS, "k").await.unwrap();
        assert_eq!(got, None);
        assert_eq!(row_count(&client, NS), 0);
    }

    #[tokio::test]
    async fn transient_rows_kept_within_ttl() {
        let dir = TempDir::new().unwrap();
        let client = SqlitePersistentCacheClient::new(dir.path().to_path_buf(), Duration::from_secs(60)).unwrap();

        client.set(NS, "k", "v", false).await.unwrap();
        let got = client.get(NS, "k").await.unwrap();
        assert_eq!(got.as_deref(), Some("v"));
    }

    #[tokio::test]
    async fn prune_runs_on_get_not_just_set() {
        let dir = TempDir::new().unwrap();
        let client = SqlitePersistentCacheClient::new(dir.path().to_path_buf(), Duration::from_millis(50)).unwrap();

        client.set(NS, "stale", "v", false).await.unwrap();
        tokio::time::sleep(Duration::from_millis(120)).await;

        // `get` on a different key still has to prune.
        let _ = client.get(NS, "other").await.unwrap();
        assert_eq!(row_count(&client, NS), 0);
    }

    #[tokio::test]
    async fn schema_migration_idempotent() {
        let dir = TempDir::new().unwrap();

        {
            let client = SqlitePersistentCacheClient::new(dir.path().to_path_buf(), Duration::from_secs(60)).unwrap();
            client.set(NS, "k", "v", true).await.unwrap();
        }

        // Reopen the same DB. `ALTER TABLE ... ADD COLUMN` will fail with
        // "duplicate column" on the second run; the client must tolerate it.
        let client = SqlitePersistentCacheClient::new(dir.path().to_path_buf(), Duration::from_secs(60)).unwrap();
        let got = client.get(NS, "k").await.unwrap();
        assert_eq!(got.as_deref(), Some("v"));
    }
}
