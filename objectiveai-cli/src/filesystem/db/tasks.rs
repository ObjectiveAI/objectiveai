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
    // `interval_seconds` is nullable: NULL means oneshot (the
    // runner fires it once on the next poll and deletes the row);
    // non-NULL means a recurring schedule with that minimum
    // interval. The CHECK still binds the non-NULL case.
    //
    // `last_ran_at` starts NULL on insert and is set by the
    // runner (#216) on each successful invocation. Recurring
    // schedules use it for the `now - last_ran_at >=
    // interval_seconds` predicate; oneshots ignore it (they fire
    // once and get deleted).
    //
    // `agent_instance_hierarchy` is denormalised from
    // `agent_arguments` so `agents tasks list` can WHERE on it
    // cheaply with depth-counted slash arithmetic. `description`
    // is the human-readable label every `schedule` invocation
    // must supply.
    // `name` is the user-facing identifier `agents tasks run` tags
    // each streamed output item with. Globally UNIQUE so the
    // tagging is unambiguous; a second `schedule` with the same
    // name surfaces a SQLite unique-constraint error to the
    // caller.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schedules (\
            id                       INTEGER PRIMARY KEY AUTOINCREMENT, \
            name                     TEXT NOT NULL UNIQUE, \
            command                  TEXT NOT NULL, \
            description              TEXT NOT NULL, \
            agent_instance_hierarchy TEXT NOT NULL, \
            interval_seconds         INTEGER CHECK (interval_seconds IS NULL OR interval_seconds >= 0), \
            agent_arguments          TEXT NOT NULL, \
            created_at               INTEGER NOT NULL, \
            last_ran_at              INTEGER \
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
    name: &str,
    command: &[String],
    description: &str,
    agent_instance_hierarchy: &str,
    interval_seconds: Option<u64>,
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
        "INSERT INTO schedules \
         (name, command, description, agent_instance_hierarchy, interval_seconds, agent_arguments, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
         RETURNING id",
    )?;
    let interval_param: Option<i64> = interval_seconds.map(|s| s as i64);
    let id = stmt.query_row(
        params![
            name,
            command_json,
            description,
            agent_instance_hierarchy,
            interval_param,
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
    name: String,
    command: Vec<String>,
    description: String,
    agent_instance_hierarchy: String,
    interval_seconds: Option<u64>,
    agent_arguments: AgentArguments,
) -> Result<i64, Error> {
    tokio::task::spawn_blocking(move || {
        insert_schedule(
            &client,
            &name,
            &command,
            &description,
            &agent_instance_hierarchy,
            interval_seconds,
            &agent_arguments,
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// One row from `schedules` as surfaced by `agents tasks list`.
/// `command` is decoded from its JSON-string column.
#[derive(Debug, Clone)]
pub struct ListedSchedule {
    pub id: i64,
    pub name: String,
    pub agent_instance_hierarchy: String,
    pub command: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub last_ran_at: Option<i64>,
    pub interval_seconds: Option<u64>,
}

/// List `schedules` matching the supplied filters. Every filter
/// is optional and composes additively — the SQL is one statement
/// that gates each predicate on whether the corresponding bind is
/// active (0 = inactive bool flag, `NULL` = unset depth/count).
///
/// * `parent` + `max_depth`: hierarchy scope. `parent` is
///   inclusive (matches itself plus descendants). `max_depth`
///   counts slashes of descent from `parent` — `Some(0)` =
///   `parent` only, `Some(1)` = parent + direct children, `None`
///   = unlimited recursion.
/// * `oneshot_only` / `interval_only`: kind filter (mutually
///   exclusive at the CLI layer; both `false` = no kind filter).
/// * `pending_only` / `exhausted_only`: readiness filter (same).
/// * `offset` / `count`: pagination. `count = None` binds `-1`
///   to LIMIT for unlimited.
pub async fn list_schedules_async(
    client: Client,
    parent: String,
    max_depth: Option<u64>,
    oneshot_only: bool,
    interval_only: bool,
    pending_only: bool,
    exhausted_only: bool,
    offset: u64,
    count: Option<u64>,
) -> Result<Vec<ListedSchedule>, Error> {
    tokio::task::spawn_blocking(move || {
        list_schedules(
            &client,
            &parent,
            max_depth,
            oneshot_only,
            interval_only,
            pending_only,
            exhausted_only,
            offset,
            count,
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn list_schedules(
    client: &Client,
    parent: &str,
    max_depth: Option<u64>,
    oneshot_only: bool,
    interval_only: bool,
    pending_only: bool,
    exhausted_only: bool,
    offset: u64,
    count: Option<u64>,
) -> Result<Vec<ListedSchedule>, Error> {
    use rusqlite::named_params;

    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem tasks db connection mutex poisoned");

    let max_depth_param: Option<i64> = max_depth.map(|d| d as i64);
    let count_param: i64 = count.map(|c| c as i64).unwrap_or(-1);
    let offset_param: i64 = offset as i64;

    let mut stmt = conn.prepare_cached(
        "SELECT id, \
                name, \
                agent_instance_hierarchy, \
                command, \
                description, \
                created_at, \
                last_ran_at, \
                interval_seconds \
         FROM schedules \
         WHERE \
             /* Hierarchy + depth filter. Inclusive of the parent itself. */ \
             ( \
                 agent_instance_hierarchy = :parent \
                 OR ( \
                     agent_instance_hierarchy LIKE (:parent || '/%') \
                     AND ( \
                         :max_depth IS NULL \
                         OR \
                         ( \
                             (length(agent_instance_hierarchy) \
                              - length(replace(agent_instance_hierarchy, '/', ''))) \
                             - (length(:parent) \
                                - length(replace(:parent, '/', ''))) \
                         ) <= :max_depth \
                     ) \
                 ) \
             ) \
             /* Oneshot / interval filter. */ \
             AND (:oneshot_only = 0 OR interval_seconds IS NULL) \
             AND (:interval_only = 0 OR interval_seconds IS NOT NULL) \
             /* Pending / exhausted filter. */ \
             AND (:pending_only = 0 OR ( \
                 (interval_seconds IS NULL AND last_ran_at IS NULL) \
                 OR \
                 (interval_seconds IS NOT NULL \
                  AND (last_ran_at IS NULL \
                       OR (:now - last_ran_at) >= interval_seconds)) \
             )) \
             AND (:exhausted_only = 0 OR ( \
                 (interval_seconds IS NULL AND last_ran_at IS NOT NULL) \
                 OR \
                 (interval_seconds IS NOT NULL \
                  AND last_ran_at IS NOT NULL \
                  AND (:now - last_ran_at) < interval_seconds) \
             )) \
         ORDER BY id ASC \
         LIMIT :count OFFSET :offset",
    )?;

    let rows = stmt
        .query_map(
            named_params! {
                ":parent": parent,
                ":max_depth": max_depth_param,
                ":oneshot_only": oneshot_only as i64,
                ":interval_only": interval_only as i64,
                ":pending_only": pending_only as i64,
                ":exhausted_only": exhausted_only as i64,
                ":now": now_seconds(),
                ":count": count_param,
                ":offset": offset_param,
            },
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = Vec::with_capacity(rows.len());
    for (id, name, hierarchy, command_json, description, created_at, last_ran_at, interval_seconds)
        in rows
    {
        let command: Vec<String> =
            serde_json::from_str(&command_json).map_err(Error::Json)?;
        out.push(ListedSchedule {
            id,
            name,
            agent_instance_hierarchy: hierarchy,
            command,
            description,
            created_at,
            last_ran_at,
            interval_seconds: interval_seconds.map(|s| s as u64),
        });
    }
    Ok(out)
}

/// A schedule row captured by `collect_and_mark_pending_async` —
/// the parts `agents tasks run` needs to fire one task. Each row
/// here has already had its `last_ran_at` bumped to `now`, and
/// (if it was a oneshot) been deleted, inside the same
/// transaction.
#[derive(Debug, Clone)]
pub struct RunRow {
    pub id: i64,
    pub name: String,
    pub command: Vec<String>,
    pub agent_arguments: AgentArguments,
}

/// Atomically: capture every pending row in scope, bump every
/// captured row's `last_ran_at = now`, and delete any captured
/// oneshots. Returns the captured rows for the caller to fire.
///
/// "Pending" means: oneshots with `last_ran_at IS NULL`, or
/// recurring rows where `now - last_ran_at >= interval_seconds`
/// (or `last_ran_at IS NULL`). Same predicate `agents tasks list
/// --pending` matches.
///
/// Updating upfront prevents a concurrent `run` from re-picking
/// the same rows; deleting oneshots upfront is the same no-retry
/// tradeoff #216's spec implies for the first-slice runner.
pub async fn collect_and_mark_pending_async(
    client: Client,
    parent: String,
    max_depth: Option<u64>,
) -> Result<Vec<RunRow>, Error> {
    tokio::task::spawn_blocking(move || {
        collect_and_mark_pending(&client, &parent, max_depth)
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn collect_and_mark_pending(
    client: &Client,
    parent: &str,
    max_depth: Option<u64>,
) -> Result<Vec<RunRow>, Error> {
    use rusqlite::named_params;

    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem tasks db connection mutex poisoned");
    let tx = conn.unchecked_transaction()?;
    let now = now_seconds();
    let max_depth_param: Option<i64> = max_depth.map(|d| d as i64);

    // 1. Capture rows.
    let captured: Vec<(i64, String, String, String)> = {
        let mut stmt = tx.prepare(
            "SELECT id, name, command, agent_arguments \
             FROM schedules \
             WHERE \
                 ( \
                     agent_instance_hierarchy = :parent \
                     OR ( \
                         agent_instance_hierarchy LIKE (:parent || '/%') \
                         AND ( \
                             :max_depth IS NULL \
                             OR \
                             ( \
                                 (length(agent_instance_hierarchy) \
                                  - length(replace(agent_instance_hierarchy, '/', ''))) \
                                 - (length(:parent) \
                                    - length(replace(:parent, '/', ''))) \
                             ) <= :max_depth \
                         ) \
                     ) \
                 ) \
                 AND ( \
                     (interval_seconds IS NULL AND last_ran_at IS NULL) \
                     OR \
                     (interval_seconds IS NOT NULL \
                      AND (last_ran_at IS NULL \
                           OR (:now - last_ran_at) >= interval_seconds)) \
                 ) \
             ORDER BY id ASC",
        )?;
        stmt.query_map(
            named_params! {
                ":parent": parent,
                ":max_depth": max_depth_param,
                ":now": now,
            },
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };

    if captured.is_empty() {
        tx.commit()?;
        return Ok(Vec::new());
    }

    // 2. Bump last_ran_at + 3. Delete oneshots. Bind ids inline
    // since rusqlite's IN-clause + dynamic vec sizes aren't
    // friendly with `params!`; loop per-id instead. The
    // transaction wraps them all so failure is atomic.
    for (id, _, _, _) in &captured {
        tx.execute(
            "UPDATE schedules SET last_ran_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        tx.execute(
            "DELETE FROM schedules WHERE id = ?1 AND interval_seconds IS NULL",
            params![id],
        )?;
    }

    tx.commit()?;

    // 4. Decode and return.
    let mut out = Vec::with_capacity(captured.len());
    for (id, name, command_json, agent_arguments_json) in captured {
        let command: Vec<String> =
            serde_json::from_str(&command_json).map_err(Error::Json)?;
        let agent_arguments: AgentArguments =
            serde_json::from_str(&agent_arguments_json).map_err(Error::Json)?;
        out.push(RunRow {
            id,
            name,
            command,
            agent_arguments,
        });
    }
    Ok(out)
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
            "schedule-a",
            &["echo".into(), "hi".into()],
            "test",
            "cli",
            Some(30),
            &AgentArguments::default(),
        )
        .unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn ids_increment_across_inserts() {
        let (c, _tmp) = fresh_client();
        let a = insert_schedule(&c, "a", &["a".into()], "", "cli", Some(1), &AgentArguments::default()).unwrap();
        let b = insert_schedule(&c, "b", &["b".into()], "", "cli", Some(1), &AgentArguments::default()).unwrap();
        let c2 = insert_schedule(&c, "c", &["c".into()], "", "cli", Some(1), &AgentArguments::default()).unwrap();
        assert!(a < b && b < c2);
    }

    #[test]
    fn zero_interval_allowed() {
        let (c, _tmp) = fresh_client();
        let id =
            insert_schedule(&c, "x", &["x".into()], "", "cli", Some(0), &AgentArguments::default()).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn oneshot_interval_null_allowed() {
        let (c, _tmp) = fresh_client();
        let id =
            insert_schedule(&c, "x", &["x".into()], "", "cli", None, &AgentArguments::default()).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn duplicate_name_rejected_by_unique_constraint() {
        let (c, _tmp) = fresh_client();
        insert_schedule(&c, "dup", &["a".into()], "", "cli", None, &AgentArguments::default()).unwrap();
        let err = insert_schedule(&c, "dup", &["b".into()], "", "cli", None, &AgentArguments::default());
        assert!(err.is_err(), "second insert with the same name must fail");
    }
}
