//! SQLite-backed persistent cache client.

use objectiveai_sdk::error::ResponseError;
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug)]
pub struct SqlitePersistentCacheClient {
    conn: Mutex<Connection>,
    /// Tracks which namespace tables have been ensured to exist.
    ensured_tables: Mutex<HashSet<&'static str>>,
}

impl SqlitePersistentCacheClient {
    pub fn new(config_base_dir: PathBuf) -> Result<Self, rusqlite::Error> {
        let db_path = config_base_dir.join("cache.sqlite");

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        Ok(Self {
            conn: Mutex::new(conn),
            ensured_tables: Mutex::new(HashSet::new()),
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
                permanent BOOLEAN NOT NULL DEFAULT 0\
            )"
        ))?;

        let mut tables = self.ensured_tables.lock().unwrap();
        tables.insert(namespace);
        Ok(())
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

        conn.execute(
            &format!("INSERT OR REPLACE INTO [{namespace}] (key, value, permanent) VALUES (?1, ?2, ?3)"),
            rusqlite::params![key, value, permanent],
        )
        .map_err(sqlite_err)?;

        Ok(())
    }
}
