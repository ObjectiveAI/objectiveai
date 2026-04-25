//! Lightweight SQLite helpers for the config module.
//!
//! Mirrors the shape of `objectiveai-api/src/ctx/persistent_cache/sqlite.rs`
//! but lives alongside the filesystem config, so both crates use the
//! same `rusqlite` version and connection conventions.
//!
//! The database file is `<base_dir>/config.sqlite`. The connection is
//! opened lazily on first use, WAL-enabled, and stored on the
//! [`super::super::Client`] so subsequent calls (including from cloned
//! clients) reuse the same handle.
//!
//! This module is intentionally minimal — just [`connection`],
//! [`execute`], [`query_one`], [`query_all`], and a re-export of the
//! `rusqlite::params!` macro. Domain-specific tables and queries
//! belong in their own modules next to their owners; this one just
//! owns the plumbing.

use std::sync::{Arc, Mutex};

pub use rusqlite::{Connection, params};

use super::super::{Client, Error};

/// Returns the shared SQLite connection for this filesystem client,
/// opening (and migrating) it if necessary.
///
/// First call per client: opens `<base_dir>/config.sqlite` (creating
/// any missing parent directories), enables WAL journaling, and
/// stashes the handle. Subsequent calls — including from cloned
/// `Client` values — return the same `Arc<Mutex<Connection>>`.
///
/// A failed open leaves the slot empty so the next call can retry,
/// rather than permanently poisoning the connection.
pub fn connection(client: &Client) -> Result<Arc<Mutex<Connection>>, Error> {
    let mut guard = client
        .db_conn_slot()
        .lock()
        .expect("config db mutex poisoned");
    if let Some(conn) = guard.as_ref() {
        return Ok(conn.clone());
    }
    let db_path = client.db_path();
    if let Some(parent) = db_path.parent() {
        // Best-effort; if the directory can't be created, `Connection::open`
        // will surface a descriptive error below.
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(&db_path)?;
    // WAL journaling allows concurrent readers + writer — matches what
    // `SqlitePersistentCacheClient` sets for the cache db.
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    let arc = Arc::new(Mutex::new(conn));
    *guard = Some(arc.clone());
    Ok(arc)
}

/// Execute a non-returning statement (`INSERT`/`UPDATE`/`DELETE`/DDL).
/// Returns the number of affected rows.
pub fn execute(
    client: &Client,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<usize, Error> {
    let conn = connection(client)?;
    let conn = conn.lock().expect("config db connection mutex poisoned");
    Ok(conn.execute(sql, params)?)
}

/// Run a `SELECT` that returns at most one row. `None` when no row
/// matched.
pub fn query_one<T, F>(
    client: &Client,
    sql: &str,
    params: impl rusqlite::Params,
    map: F,
) -> Result<Option<T>, Error>
where
    F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let conn = connection(client)?;
    let conn = conn.lock().expect("config db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(sql)?;
    use rusqlite::OptionalExtension as _;
    Ok(stmt.query_row(params, map).optional()?)
}

/// Run a `SELECT` that returns zero or more rows.
pub fn query_all<T, F>(
    client: &Client,
    sql: &str,
    params: impl rusqlite::Params,
    mut map: F,
) -> Result<Vec<T>, Error>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let conn = connection(client)?;
    let conn = conn.lock().expect("config db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(params, |row| map(row))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
