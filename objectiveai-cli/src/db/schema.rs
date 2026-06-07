//! `messages` / `files` table primitives. Postgres-backed via sqlx.
//!
//! Pure helpers (`message_kind_as_str`, `parse_message_kind`,
//! `message_kind_file_path`, `MessageRow`) carry over verbatim from
//! the sqlite predecessor; the rest take `&Pool` and run native-async
//! against the embedded postmaster.

use objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind;
use sqlx::Row as _;

use super::{Error, Pool};

/// TEXT-column form of `kind`, produced and consumed by every row
/// insert/read. Canonical inverse of [`parse_message_kind`].
pub fn message_kind_as_str(kind: RequestMessageKind) -> &'static str {
    match kind {
        RequestMessageKind::AgentCompletionRequest => "agent_completion_request",
        RequestMessageKind::FunctionExecutionRequest => "function_execution_request",
        RequestMessageKind::FunctionInventionRecursiveRequest => {
            "function_invention_recursive_request"
        }
        RequestMessageKind::AgentCompletionNotification => "agent_completion_notification",
        RequestMessageKind::AssistantResponse => "assistant_response",
        RequestMessageKind::ToolResponse => "tool_response",
    }
}

/// Parse the TEXT representation produced by [`message_kind_as_str`].
pub fn parse_message_kind(s: &str) -> Result<RequestMessageKind, Error> {
    match s {
        "agent_completion_request" => Ok(RequestMessageKind::AgentCompletionRequest),
        "function_execution_request" => Ok(RequestMessageKind::FunctionExecutionRequest),
        "function_invention_recursive_request" => {
            Ok(RequestMessageKind::FunctionInventionRecursiveRequest)
        }
        "agent_completion_notification" => {
            Ok(RequestMessageKind::AgentCompletionNotification)
        }
        "assistant_response" => Ok(RequestMessageKind::AssistantResponse),
        "tool_response" => Ok(RequestMessageKind::ToolResponse),
        other => Err(Error::InvalidData(format!("unknown message kind: {other}"))),
    }
}

/// Reconstruct the on-disk file path (relative to `logs_dir`) from a
/// (kind, response_id, path) row.
///
/// `response_id` is the bare agent-completion chunk id and is passed in
/// explicitly. We do **not** recover it by parsing
/// `agent_instance_hierarchy`'s trailing segment.
///
/// Request rows have `path` already set to the full filesystem path
/// (the writer stores it as `agents/completions/request/<id>.json`);
/// we return it verbatim. The fallback `{prefix}/{path}.json` form
/// keeps backwards compat for any bare-stem rows.
pub fn message_kind_file_path(
    kind: RequestMessageKind,
    response_id: &str,
    path: &str,
) -> String {
    match kind {
        RequestMessageKind::AgentCompletionRequest => {
            if path.starts_with("agents/completions/request/") && path.ends_with(".json") {
                path.to_string()
            } else {
                format!("agents/completions/request/{path}.json")
            }
        }
        RequestMessageKind::FunctionExecutionRequest => {
            if path.starts_with("functions/executions/request/")
                && path.ends_with(".json")
            {
                path.to_string()
            } else {
                format!("functions/executions/request/{path}.json")
            }
        }
        RequestMessageKind::FunctionInventionRecursiveRequest => {
            if path.starts_with("functions/inventions/recursive/request/")
                && path.ends_with(".json")
            {
                path.to_string()
            } else {
                format!("functions/inventions/recursive/request/{path}.json")
            }
        }
        RequestMessageKind::AssistantResponse => {
            format!(
                "agents/completions/response/messages/assistant/{response_id}_{path}.json"
            )
        }
        RequestMessageKind::ToolResponse => {
            format!(
                "agents/completions/response/messages/tool/{response_id}_{path}.json"
            )
        }
        RequestMessageKind::AgentCompletionNotification => {
            format!(
                "agents/completions/request/notifications/{response_id}_{path}.json"
            )
        }
    }
}

/// A single row to be inserted into the `messages` table. Produced by
/// chunk types' `produce_message_rows()`.
#[derive(Debug, Clone)]
pub struct MessageRow {
    /// Which agent the row is about (column). Lineage-stamped by the
    /// writer (`{caller}/{response_id}` or just `{response_id}` at the
    /// root).
    pub agent_instance_hierarchy: String,
    /// The bare chunk id (the agent completion's response id). Set
    /// explicitly by the producer.
    pub response_id: String,
    pub kind: RequestMessageKind,
    /// Chunk-given message index (assistant/tool: `MessageChunk::index()`).
    pub index: u64,
    /// Bare-id placed in the `path` column. See [`message_kind_file_path`]
    /// for the full filesystem path reconstruction.
    pub path: String,
    /// Unix seconds; usually the chunk's `created` field.
    pub timestamp: u64,
}

/// Look up (or insert) the SQL row id for `path` in the `files` table.
/// Idempotent: same path → same id forever, even across processes and
/// concurrent callers, because of the `UNIQUE(path)` constraint.
///
/// Uses `INSERT … ON CONFLICT(path) DO UPDATE SET path=EXCLUDED.path
/// RETURNING id`. The no-op `UPDATE` makes `RETURNING` fire on the
/// existing row when there's a conflict, so the call always returns
/// the right id in one round-trip.
pub async fn file_id_for_path(pool: &Pool, path: &str) -> Result<i64, Error> {
    let row = sqlx::query(
        "INSERT INTO files (path) VALUES ($1) \
         ON CONFLICT (path) DO UPDATE SET path = EXCLUDED.path \
         RETURNING id",
    )
    .bind(path)
    .fetch_one(&**pool)
    .await?;
    Ok(row.try_get::<i64, _>(0)?)
}

/// Resolve a SQL row id back to its path. `None` when no row matches.
pub async fn path_for_file_id(pool: &Pool, id: i64) -> Result<Option<String>, Error> {
    let row = sqlx::query("SELECT path FROM files WHERE id = $1")
        .bind(id)
        .fetch_optional(&**pool)
        .await?;
    Ok(row.map(|r| r.try_get::<String, _>(0)).transpose()?)
}

/// List every direct-child agent of `parent_agent_instance_hierarchy`
/// (one lineage segment deeper, no grandchildren) along with the unix-
/// seconds timestamp of its most recent
/// [`RequestMessageKind::AssistantResponse`] row. Newest-first.
///
/// Composite agent ids are slash-separated lineage strings; "direct
/// child" means: `LIKE 'parent/%'` AND no further `/` after the prefix
/// (`position('/' in substring(hierarchy from parent_len+2)) = 0`).
pub async fn list_direct_active_children(
    pool: &Pool,
    parent_agent_instance_hierarchy: &str,
) -> Result<Vec<(String, u64)>, Error> {
    // postgres's `position('/' in <substr>)` returns 1-based position
    // or 0 when not found — the `= 0` test is the "no further slash"
    // gate. `substring(s from n)` is the postgres equivalent of
    // sqlite's `substr(s, n)`. The `+ 2` accounts for the extra
    // boundary slash after `parent` (1-based indexing).
    let prefix_len = parent_agent_instance_hierarchy.len() as i32;
    let pattern = format!("{parent_agent_instance_hierarchy}/%");
    let rows = sqlx::query(
        "SELECT agent_instance_hierarchy, MAX(timestamp) AS last_log \
         FROM messages \
         WHERE agent_instance_hierarchy LIKE $1 \
           AND position('/' in substring(agent_instance_hierarchy from cast($2 as int) + 2)) = 0 \
           AND kind = $3 \
         GROUP BY agent_instance_hierarchy \
         ORDER BY last_log DESC",
    )
    .bind(&pattern)
    .bind(prefix_len)
    .bind(message_kind_as_str(RequestMessageKind::AssistantResponse))
    .fetch_all(&**pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let hierarchy: String = row.try_get(0)?;
        let last_log: i64 = row.try_get(1)?;
        out.push((hierarchy, last_log.max(0) as u64));
    }
    Ok(out)
}

/// `SELECT MAX("index") FROM messages WHERE agent_instance_hierarchy = ?`.
/// `None` when no row matches.
pub async fn max_index(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<Option<u64>, Error> {
    let row = sqlx::query(
        "SELECT MAX(\"index\") FROM messages WHERE agent_instance_hierarchy = $1",
    )
    .bind(agent_instance_hierarchy)
    .fetch_one(&**pool)
    .await?;
    let v: Option<i64> = row.try_get(0)?;
    Ok(v.map(|x| x.max(0) as u64))
}

/// Whether `agent_instance_hierarchy` has any
/// `agent_completion_request` row logged in the messages table.
pub async fn agent_exists(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<bool, Error> {
    let row = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM messages \
         WHERE agent_instance_hierarchy = $1 AND kind = $2)",
    )
    .bind(agent_instance_hierarchy)
    .bind(message_kind_as_str(RequestMessageKind::AgentCompletionRequest))
    .fetch_one(&**pool)
    .await?;
    Ok(row.try_get::<bool, _>(0)?)
}

/// Insert a single row into `messages`.
///
/// `agent_instance_hierarchy` is the lineage-stamped composite (`{caller}/{response_id}`
/// or just `{response_id}` for the unstamped root case). `response_id`
/// is the *bare* chunk id, passed in explicitly — we never recover it
/// by parsing `agent_instance_hierarchy`.
pub async fn insert(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    response_id: &str,
    kind: RequestMessageKind,
    path: &str,
    timestamp: u64,
    index: u64,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO messages (agent_instance_hierarchy, response_id, kind, path, timestamp, \"index\") \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(agent_instance_hierarchy)
    .bind(response_id)
    .bind(message_kind_as_str(kind))
    .bind(path)
    .bind(timestamp as i64)
    .bind(index as i64)
    .execute(&**pool)
    .await?;
    Ok(())
}

/// Every `AgentCompletionRequest` row's `path` column for
/// `agent_instance_hierarchy`, ordered by `"index"` descending — newest
/// first. The `path` column carries the full on-disk path
/// (`agents/completions/request/<id>.json`); the caller is responsible
/// for stripping the prefix + `.json` to recover the bare response id.
pub async fn agent_completion_request_paths_newest_first(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    let rows = sqlx::query(
        "SELECT path FROM messages \
         WHERE agent_instance_hierarchy = $1 AND kind = $2 \
         ORDER BY \"index\" DESC",
    )
    .bind(agent_instance_hierarchy)
    .bind(message_kind_as_str(RequestMessageKind::AgentCompletionRequest))
    .fetch_all(&**pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(row.try_get::<String, _>(0)?);
    }
    Ok(out)
}
