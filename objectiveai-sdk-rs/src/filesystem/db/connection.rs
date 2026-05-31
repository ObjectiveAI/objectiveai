//! Shared SQLite plumbing for the filesystem.
//!
//! One `db.sqlite` per `Client` at `<base_dir>/db.sqlite`, opened
//! lazily on first use, WAL-enabled, with the `messages` table from
//! [`super::schema`] ensured at open time.
//!
//! `Client` carries the lazy-init slot in `db_conn`; subsequent calls
//! (including from cloned `Client` values) return the same
//! `Arc<Mutex<Connection>>`.
//!
//! A failed open leaves the slot empty so the next call can retry,
//! rather than permanently poisoning the connection.

use std::sync::{Arc, Mutex};

pub use rusqlite::{Connection, params};

use super::super::{Client, Error};

/// Returns the shared SQLite connection for this filesystem client,
/// opening (and initialising) it if necessary.
pub fn connection(client: &Client) -> Result<Arc<Mutex<Connection>>, Error> {
    let mut guard = client
        .db_conn_slot()
        .lock()
        .expect("filesystem db mutex poisoned");
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
    // WAL journaling allows concurrent readers + writer — readers see
    // a snapshot, the single writer appends to the WAL without blocking.
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    // Ensure the `messages` and `messages_queue` tables exist.
    super::schema::init_tables(&conn)?;
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
    let conn = conn.lock().expect("filesystem db connection mutex poisoned");
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
    let conn = conn.lock().expect("filesystem db connection mutex poisoned");
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
    let conn = conn.lock().expect("filesystem db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(params, |row| map(row))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
