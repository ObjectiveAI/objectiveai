//! Deferred-message storage for `agents enqueue`.
//!
//! Lives in a dedicated `prompt_queue.sqlite` file under the
//! base dir, mirroring the `tags.sqlite` separate-file pattern so
//! the enqueue lifecycle stays isolated from the main message-log
//! database.
//!
//! ## Schema
//!
//! One table, `prompts`. Each row targets either an
//! `agent_instance_hierarchy` OR an `agent_tag`, never both
//! (enforced by `CHECK`). Tags are stored verbatim — no resolution
//! at enqueue time; a future reader will resolve at dequeue time.
//!
//! Atomic dequeue via `DELETE … RETURNING …` is the planned future
//! shape; the `(target, id)` partial indexes are sized for it.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use super::super::{Client, Error};

/// Returns the shared SQLite connection for this filesystem client's
/// dedicated `prompt_queue.sqlite`, opening (and initialising) it if
/// necessary. Same lazy-init / no-poison semantics as
/// [`super::tags::connection`].
pub fn connection(client: &Client) -> Result<Arc<Mutex<Connection>>, Error> {
    let mut guard = client
        .prompt_queue_db_conn_slot()
        .lock()
        .expect("filesystem prompt-queue db mutex poisoned");
    if let Some(conn) = guard.as_ref() {
        return Ok(conn.clone());
    }
    let db_path = client.prompt_queue_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    init_table(&conn)?;
    let arc = Arc::new(Mutex::new(conn));
    *guard = Some(arc.clone());
    Ok(arc)
}

fn init_table(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS prompts (\
            id                       INTEGER PRIMARY KEY AUTOINCREMENT, \
            agent_instance_hierarchy TEXT, \
            agent_tag                TEXT, \
            prompt                   TEXT NOT NULL, \
            enqueued_at              INTEGER NOT NULL, \
            CHECK ( \
                (agent_instance_hierarchy IS NOT NULL AND agent_tag IS NULL) \
                OR \
                (agent_instance_hierarchy IS NULL AND agent_tag IS NOT NULL) \
            ) \
        );\
        CREATE INDEX IF NOT EXISTS prompts_hierarchy_idx \
            ON prompts(agent_instance_hierarchy, id) \
            WHERE agent_instance_hierarchy IS NOT NULL;\
        CREATE INDEX IF NOT EXISTS prompts_tag_idx \
            ON prompts(agent_tag, id) \
            WHERE agent_tag IS NOT NULL;",
    )?;
    Ok(())
}

fn now_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Insert one row. Exactly one of `agent_instance_hierarchy` or
/// `agent_tag` must be `Some` (the table's `CHECK` constraint will
/// reject malformed rows at the DB layer). Returns the auto-
/// incremented `id` of the new row via `INSERT ... RETURNING id`.
pub fn insert(
    client: &Client,
    agent_instance_hierarchy: Option<&str>,
    agent_tag: Option<&str>,
    prompt: &str,
) -> Result<i64, Error> {
    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompt-queue db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(
        "INSERT INTO prompts (agent_instance_hierarchy, agent_tag, prompt, enqueued_at) \
         VALUES (?1, ?2, ?3, ?4) \
         RETURNING id",
    )?;
    let id = stmt.query_row(
        rusqlite::params![agent_instance_hierarchy, agent_tag, prompt, now_seconds()],
        |r| r.get::<_, i64>(0),
    )?;
    Ok(id)
}

/// Async wrapper around [`insert`].
pub async fn insert_async(
    client: Client,
    agent_instance_hierarchy: Option<String>,
    agent_tag: Option<String>,
    prompt: String,
) -> Result<i64, Error> {
    tokio::task::spawn_blocking(move || {
        insert(
            &client,
            agent_instance_hierarchy.as_deref(),
            agent_tag.as_deref(),
            &prompt,
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn spawn_blocking_join_err(e: tokio::task::JoinError) -> Error {
    Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_client() -> (Client, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let client = Client::new(
            Some(tmp.path().to_path_buf()),
            Some("test"),
            Some("test@test"),
        );
        (client, tmp)
    }

    #[test]
    fn insert_direct_row_returns_id_starting_at_one() {
        let (c, _tmp) = fresh_client();
        let id = insert(&c, Some("root/A/inst-1"), None, "[]").unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn insert_tag_row_returns_id() {
        let (c, _tmp) = fresh_client();
        let id = insert(&c, None, Some("foo"), "[]").unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn insert_with_both_targets_violates_check() {
        let (c, _tmp) = fresh_client();
        let err = insert(&c, Some("root/A"), Some("foo"), "[]");
        assert!(err.is_err(), "CHECK constraint must reject both columns set");
    }

    #[test]
    fn insert_with_neither_target_violates_check() {
        let (c, _tmp) = fresh_client();
        let err = insert(&c, None, None, "[]");
        assert!(err.is_err(), "CHECK constraint must reject neither column set");
    }

    #[test]
    fn ids_increment_across_inserts() {
        let (c, _tmp) = fresh_client();
        let a = insert(&c, Some("root/A/h1"), None, "[]").unwrap();
        let b = insert(&c, None, Some("t"), "[]").unwrap();
        let c2 = insert(&c, Some("root/A/h2"), None, "[]").unwrap();
        assert!(a < b && b < c2);
    }
}
