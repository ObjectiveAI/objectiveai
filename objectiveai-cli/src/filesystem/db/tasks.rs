//! `tasks.sqlite` — schedules + future task-runner tables.
//!
//! Per-row payload today (single `schedules` table): an argv vector
//! to invoke on each scheduled poll, the minimum interval between
//! invocations in seconds, and a JSON snapshot of the caller's
//! `AgentArguments` so the runner can re-install identity env vars
//! at fire-time. Adding only; the runner that actually fires
//! schedules is follow-up work (#216).

use std::sync::{Arc, Mutex};

use objectiveai_sdk::cli::command::AgentArguments;
use rusqlite::{Connection, params};

use super::super::{Client, Error};

/// Returns the shared SQLite connection for this filesystem client's
/// dedicated `tasks.sqlite`, opening (and initialising) it if necessary.
/// Same lazy-init / no-poison semantics as
/// [`super::tags::connection`].
pub fn connection(client: &Client) -> Result<Arc<Mutex<Connection>>, Error> {
    let mut guard = client
        .tasks_db_conn_slot()
        .lock()
        .expect("filesystem tasks db mutex poisoned");
    if let Some(conn) = guard.as_ref() {
        return Ok(conn.clone());
    }
    let db_path = client.tasks_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    init_tables(&conn)?;
    let arc = Arc::new(Mutex::new(conn));
    *guard = Some(arc.clone());
    Ok(arc)
}

/// Create every table this DB hosts. Today: just `schedules`.
fn init_tables(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schedules (\
            id               INTEGER PRIMARY KEY AUTOINCREMENT, \
            command          TEXT NOT NULL, \
            interval_seconds INTEGER NOT NULL CHECK (interval_seconds >= 0), \
            agent_arguments  TEXT NOT NULL, \
            created_at       INTEGER NOT NULL \
        );",
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

/// Insert one schedule row and return its auto-incremented id.
///
/// `command` is JSON-serialised as a string array (the argv shape
/// the runner will exec). `agent_arguments` is JSON-serialised
/// verbatim — the runner re-installs each `Some(_)` field as the
/// matching env var when the schedule fires.
pub fn insert_schedule(
    client: &Client,
    command: &[String],
    interval_seconds: u64,
    agent_arguments: &AgentArguments,
) -> Result<i64, Error> {
    let command_json = serde_json::to_string(command).map_err(Error::Json)?;
    let agent_arguments_json =
        serde_json::to_string(agent_arguments).map_err(Error::Json)?;
    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem tasks db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(
        "INSERT INTO schedules (command, interval_seconds, agent_arguments, created_at) \
         VALUES (?1, ?2, ?3, ?4) \
         RETURNING id",
    )?;
    let id = stmt.query_row(
        params![
            command_json,
            interval_seconds as i64,
            agent_arguments_json,
            now_seconds()
        ],
        |r| r.get::<_, i64>(0),
    )?;
    Ok(id)
}

/// Async wrapper around [`insert_schedule`].
pub async fn insert_schedule_async(
    client: Client,
    command: Vec<String>,
    interval_seconds: u64,
    agent_arguments: AgentArguments,
) -> Result<i64, Error> {
    tokio::task::spawn_blocking(move || {
        insert_schedule(&client, &command, interval_seconds, &agent_arguments)
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
    fn insert_schedule_returns_id_starting_at_one() {
        let (c, _tmp) = fresh_client();
        let id = insert_schedule(
            &c,
            &["echo".into(), "hi".into()],
            30,
            &AgentArguments::default(),
        )
        .unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn ids_increment_across_inserts() {
        let (c, _tmp) = fresh_client();
        let a = insert_schedule(&c, &["a".into()], 1, &AgentArguments::default()).unwrap();
        let b = insert_schedule(&c, &["b".into()], 1, &AgentArguments::default()).unwrap();
        let c2 = insert_schedule(&c, &["c".into()], 1, &AgentArguments::default()).unwrap();
        assert!(a < b && b < c2);
    }

    #[test]
    fn zero_interval_allowed() {
        let (c, _tmp) = fresh_client();
        let id =
            insert_schedule(&c, &["x".into()], 0, &AgentArguments::default()).unwrap();
        assert!(id > 0);
    }
}
