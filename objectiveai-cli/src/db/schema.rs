//! Per-message-row primitives + the `messages` / `files` tables'
//! `&Pool`-accepting API. Bodies are stubbed; real SQL lands in
//! stage 5.

use objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind;

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
pub async fn file_id_for_path(_pool: &Pool, _path: &str) -> Result<i64, Error> {
    unimplemented!("db::schema::file_id_for_path — stage 5")
}

/// Resolve a SQL row id back to its path.
pub async fn path_for_file_id(_pool: &Pool, _id: i64) -> Result<Option<String>, Error> {
    unimplemented!("db::schema::path_for_file_id — stage 5")
}

/// List every direct-child agent of `parent_agent_instance_hierarchy`
/// along with the unix-seconds timestamp of its most recent
/// `AssistantResponse` row. Newest-first.
pub async fn list_direct_active_children(
    _pool: &Pool,
    _parent_agent_instance_hierarchy: &str,
) -> Result<Vec<(String, u64)>, Error> {
    unimplemented!("db::schema::list_direct_active_children — stage 5")
}

/// `SELECT MAX("index") FROM messages WHERE agent_instance_hierarchy = ?`.
pub async fn max_index(
    _pool: &Pool,
    _agent_instance_hierarchy: &str,
) -> Result<Option<u64>, Error> {
    unimplemented!("db::schema::max_index — stage 5")
}

/// Whether the cli has ever logged an `agent_completion_request` row
/// against `agent_instance_hierarchy`.
pub async fn agent_exists(
    _pool: &Pool,
    _agent_instance_hierarchy: &str,
) -> Result<bool, Error> {
    unimplemented!("db::schema::agent_exists — stage 5")
}

/// Insert a single message row.
pub async fn insert(
    _pool: &Pool,
    _agent_instance_hierarchy: &str,
    _response_id: &str,
    _kind: RequestMessageKind,
    _path: &str,
    _timestamp: u64,
    _index: u64,
) -> Result<(), Error> {
    unimplemented!("db::schema::insert — stage 5")
}

/// Every `AgentCompletionRequest` row's `path` column for
/// `agent_instance_hierarchy`, ordered by `"index"` descending —
/// newest first. The path column carries the full on-disk path
/// (`agents/completions/request/<id>.json`); strip the prefix +
/// `.json` to recover the bare response id.
pub async fn agent_completion_request_paths_newest_first(
    _pool: &Pool,
    _agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    unimplemented!(
        "db::schema::agent_completion_request_paths_newest_first — stage 5"
    )
}
