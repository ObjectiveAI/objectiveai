//! `messages` table schema + sync/async sqlite primitives. The
//! [`super::messages::Queue`] is the intended caller for the
//! primitives; nothing else in the workspace should poke at them
//! directly.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

/// Discriminant for the row's payload. Persisted as TEXT via the
/// `as_str()` mapping so on-disk dumps stay readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageKind {
    AgentCompletionRequest,
    FunctionExecutionRequest,
    FunctionInventionRecursiveRequest,
    /// Notifications drained from the per-agent socket and prepended
    /// to the next tool response (or written at stream end if none).
    AgentCompletionNotification,
    AssistantResponse,
    ToolResponse,
}

impl MessageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageKind::AgentCompletionRequest => "agent_completion_request",
            MessageKind::FunctionExecutionRequest => "function_execution_request",
            MessageKind::FunctionInventionRecursiveRequest => "function_invention_recursive_request",
            MessageKind::AgentCompletionNotification => "agent_completion_notification",
            MessageKind::AssistantResponse => "assistant_response",
            MessageKind::ToolResponse => "tool_response",
        }
    }

    /// Parse the TEXT representation produced by [`Self::as_str`]
    /// back into a `MessageKind`. Errors with
    /// `Error::InvalidPath(format!("unknown message kind: {}", s))`
    /// on an unrecognised string — mainly a guard against
    /// out-of-sync rows from a future schema.
    pub fn from_str(s: &str) -> Result<Self, super::super::Error> {
        match s {
            "agent_completion_request" => Ok(MessageKind::AgentCompletionRequest),
            "function_execution_request" => Ok(MessageKind::FunctionExecutionRequest),
            "function_invention_recursive_request" => Ok(MessageKind::FunctionInventionRecursiveRequest),
            "agent_completion_notification" => Ok(MessageKind::AgentCompletionNotification),
            "assistant_response" => Ok(MessageKind::AssistantResponse),
            "tool_response" => Ok(MessageKind::ToolResponse),
            other => Err(super::super::Error::InvalidPath(format!(
                "unknown message kind: {other}"
            ))),
        }
    }

    /// Reconstruct the on-disk file path (relative to `logs_dir`)
    /// from a (kind, response_id, path) row.
    ///
    /// `response_id` is the bare agent-completion chunk id and is
    /// passed in explicitly. We do **not** recover it by parsing
    /// `agent_id`'s trailing segment — `agent_id` is constructed
    /// from `response_id` (by lineage-stamping) and the reverse
    /// direction is unsafe (bare/unstamped agent_ids, sub-lineages,
    /// etc.). For notifications, the on-disk filename is keyed by
    /// `response_id` too (the target agent-completion's id) so the
    /// rule holds uniformly.
    ///
    /// Request rows have `path` already set to the full filesystem
    /// path (the writer stores it as
    /// `agents/completions/request/<id>.json`); we return it
    /// verbatim. The fallback `{prefix}/{path}.json` form keeps
    /// backwards compat for any bare-stem rows.
    pub fn file_path(&self, response_id: &str, path: &str) -> String {
        match self {
            MessageKind::AgentCompletionRequest => {
                if path.starts_with("agents/completions/request/")
                    && path.ends_with(".json")
                {
                    path.to_string()
                } else {
                    format!("agents/completions/request/{path}.json")
                }
            }
            MessageKind::FunctionExecutionRequest => {
                if path.starts_with("functions/executions/request/")
                    && path.ends_with(".json")
                {
                    path.to_string()
                } else {
                    format!("functions/executions/request/{path}.json")
                }
            }
            MessageKind::FunctionInventionRecursiveRequest => {
                if path.starts_with("functions/inventions/recursive/request/")
                    && path.ends_with(".json")
                {
                    path.to_string()
                } else {
                    format!("functions/inventions/recursive/request/{path}.json")
                }
            }
            MessageKind::AssistantResponse => {
                format!(
                    "agents/completions/response/messages/assistant/{response_id}_{path}.json"
                )
            }
            MessageKind::ToolResponse => {
                format!(
                    "agents/completions/response/messages/tool/{response_id}_{path}.json"
                )
            }
            MessageKind::AgentCompletionNotification => {
                format!(
                    "agents/completions/request/notifications/{response_id}_{path}.json"
                )
            }
        }
    }
}

/// A single row to be inserted into the `messages` table. Produced
/// by chunk types' `produce_message_rows()`.
#[derive(Debug, Clone)]
pub struct MessageRow {
    /// Which agent the row is about (column). Lineage-stamped by the
    /// writer (`{caller}/{response_id}` or just `{response_id}` at the
    /// root).
    pub agent_id: String,
    /// The bare chunk id (the agent completion's response id). Set
    /// explicitly by the producer; never re-derived from `agent_id`.
    pub response_id: String,
    pub kind: MessageKind,
    /// The chunk-given message index (assistant/tool: `MessageChunk::index()`).
    pub index: u64,
    /// Bare-id placed in the `path` column. See [`MessageKind::file_path`]
    /// for the full filesystem path reconstruction.
    pub path: String,
    /// Unix seconds; usually the chunk's `created` field.
    pub timestamp: u64,
}

/// Create every table the shared db uses if it doesn't already
/// exist. Called from [`super::connection::connection`] on first
/// open of `db.sqlite`.
///
/// Tables:
/// - `messages` — one row per request / response / notification.
/// - `messages_queue` — per-`(caller_agent_id, spawned_agent_id)`
///   watermark of the highest `messages."index"` the caller has
///   already consumed. One row per pair; the composite PRIMARY KEY
///   doubles as the lookup index.
/// - `files` — lazy id↔path table populated on-demand by
///   `read_new_from_queue` so callers can hold compact integer
///   ids instead of full path strings. `UNIQUE(path)` enforces a
///   stable one-to-one mapping forever.
pub fn init_tables(conn: &Connection) -> Result<(), super::super::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS messages (\
            id          INTEGER PRIMARY KEY AUTOINCREMENT, \
            agent_id    TEXT NOT NULL, \
            response_id TEXT NOT NULL, \
            kind        TEXT NOT NULL, \
            path        TEXT NOT NULL, \
            timestamp   INTEGER NOT NULL, \
            \"index\"   INTEGER NOT NULL\
        );\
        CREATE INDEX IF NOT EXISTS messages_agent_index_idx ON messages(agent_id, \"index\");\
        CREATE INDEX IF NOT EXISTS messages_agent_idx ON messages(agent_id);\
        CREATE TABLE IF NOT EXISTS messages_queue (\
            caller_agent_id  TEXT NOT NULL, \
            spawned_agent_id TEXT NOT NULL, \
            \"index\"        INTEGER NOT NULL, \
            PRIMARY KEY (caller_agent_id, spawned_agent_id)\
        );\
        CREATE TABLE IF NOT EXISTS files (\
            id   INTEGER PRIMARY KEY AUTOINCREMENT, \
            path TEXT NOT NULL UNIQUE\
        );",
    )?;
    Ok(())
}

/// Look up (or insert) the SQL row id for `path` in the `files`
/// table. Idempotent: same path → same id forever, even across
/// processes and concurrent callers, because of the `UNIQUE(path)`
/// constraint.
///
/// Uses `INSERT … ON CONFLICT(path) DO UPDATE SET path=excluded.path
/// RETURNING id`. The no-op `UPDATE` is the canonical SQLite trick
/// that makes `RETURNING` fire on the existing row when there's a
/// conflict, so the call always returns the right id in one round-
/// trip.
pub fn file_id_for_path(
    conn: &Connection,
    path: &str,
) -> Result<i64, super::super::Error> {
    let mut stmt = conn.prepare_cached(
        "INSERT INTO files (path) VALUES (?1) \
         ON CONFLICT(path) DO UPDATE SET path = excluded.path \
         RETURNING id",
    )?;
    Ok(stmt.query_row([path], |r| r.get::<_, i64>(0))?)
}

/// Resolve a SQL row id back to its path. `None` when no row matches.
pub fn path_for_file_id(
    conn: &Connection,
    id: i64,
) -> Result<Option<String>, super::super::Error> {
    use rusqlite::OptionalExtension as _;
    let mut stmt = conn.prepare_cached("SELECT path FROM files WHERE id = ?1")?;
    Ok(stmt.query_row([id], |r| r.get::<_, String>(0)).optional()?)
}

/// Async wrapper around [`file_id_for_path`].
pub async fn file_id_for_path_async(
    conn: Arc<Mutex<Connection>>,
    path: String,
) -> Result<i64, super::super::Error> {
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().expect("filesystem db mutex poisoned");
        file_id_for_path(&conn, &path)
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// Async wrapper around [`path_for_file_id`].
pub async fn path_for_file_id_async(
    conn: Arc<Mutex<Connection>>,
    id: i64,
) -> Result<Option<String>, super::super::Error> {
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().expect("filesystem db mutex poisoned");
        path_for_file_id(&conn, id)
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// List every direct-child agent of `parent_agent_id` (one
/// lineage segment deeper, no grandchildren) along with the unix-
/// seconds timestamp of its most recent
/// [`MessageKind::AssistantResponse`] row. Newest-first.
///
/// Composite agent ids are slash-separated lineage strings minted
/// at the api server (`{parent}/{local_id}`). "Direct child"
/// means: `LIKE 'parent/%'` AND no further `/` after the prefix.
pub fn list_direct_active_children(
    conn: &Connection,
    parent_agent_id: &str,
) -> Result<Vec<(String, u64)>, super::super::Error> {
    let mut stmt = conn.prepare_cached(
        "SELECT agent_id, MAX(timestamp) AS last_log \
         FROM messages \
         WHERE agent_id LIKE (?1 || '/%') \
           AND instr(substr(agent_id, length(?1) + 2), '/') = 0 \
           AND kind = ?2 \
         GROUP BY agent_id \
         ORDER BY last_log DESC",
    )?;
    let rows = stmt
        .query_map(
            rusqlite::params![parent_agent_id, MessageKind::AssistantResponse.as_str()],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?.max(0) as u64)),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Async wrapper around [`list_direct_active_children`].
pub async fn list_direct_active_children_async(
    conn: Arc<Mutex<Connection>>,
    parent_agent_id: String,
) -> Result<Vec<(String, u64)>, super::super::Error> {
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().expect("filesystem db mutex poisoned");
        list_direct_active_children(&conn, &parent_agent_id)
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// `SELECT MAX("index") FROM messages WHERE agent_id = ?`. `None` when
/// no row matches.
pub fn max_index(
    conn: &Connection,
    agent_id: &str,
) -> Result<Option<u64>, super::super::Error> {
    let mut stmt = conn
        .prepare_cached("SELECT MAX(\"index\") FROM messages WHERE agent_id = ?1")?;
    use rusqlite::OptionalExtension as _;
    let row: Option<Option<i64>> = stmt
        .query_row([agent_id], |r| r.get::<_, Option<i64>>(0))
        .optional()?;
    Ok(row.flatten().map(|v| v.max(0) as u64))
}

/// Insert a single row.
///
/// `agent_id` is the lineage-stamped composite (`{caller}/{response_id}`
/// or just `{response_id}` for the unstamped root case). `response_id`
/// is the *bare* chunk id, passed in explicitly — we never recover it
/// by parsing `agent_id`, both because `agent_id` may be unstamped and
/// because the trailing-segment trick is a one-way invariant that
/// callers shouldn't rely on.
pub fn insert(
    conn: &Connection,
    agent_id: &str,
    response_id: &str,
    kind: MessageKind,
    path: &str,
    timestamp: u64,
    index: u64,
) -> Result<(), super::super::Error> {
    conn.execute(
        "INSERT INTO messages (agent_id, response_id, kind, path, timestamp, \"index\") \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            agent_id,
            response_id,
            kind.as_str(),
            path,
            timestamp as i64,
            index as i64
        ],
    )?;
    Ok(())
}

/// Async wrapper: insert one message row on the blocking pool. Locks
/// the connection only inside the blocking body so the lock never
/// crosses an `.await`.
pub async fn insert_async(
    conn: Arc<Mutex<Connection>>,
    agent_id: String,
    response_id: String,
    kind: MessageKind,
    path: String,
    timestamp: u64,
    index: u64,
) -> Result<(), super::super::Error> {
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().expect("filesystem db mutex poisoned");
        insert(&conn, &agent_id, &response_id, kind, &path, timestamp, index)
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// Async wrapper: `SELECT MAX("index") WHERE agent_id = ?` on the
/// blocking pool.
pub async fn max_index_async(
    conn: Arc<Mutex<Connection>>,
    agent_id: String,
) -> Result<Option<u64>, super::super::Error> {
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().expect("filesystem db mutex poisoned");
        max_index(&conn, &agent_id)
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn spawn_blocking_join_err(e: tokio::task::JoinError) -> super::super::Error {
    super::super::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
}
