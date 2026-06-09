//! `agents logs read all` / `agents logs read pending` backend:
//! SELECT `logs.messages` rows for a target AIH (or every child
//! AIH of a parent), coalesce consecutive rows into `ResponseItem`
//! blocks, and yield them in index order.
//!
//! Block-coalesce rule: a new block starts when ANY of
//! `(block_class, agent_instance_hierarchy, response_id)` changes
//! between two adjacent rows. The three request-blob classes are
//! always single-row blocks. Within a multi-row block, every part
//! shares the same agent_instance_hierarchy and response_id by
//! construction.
//!
//! `read pending` is read-and-advance, expressed as a single
//! CTE-chained SQL statement: the SELECT returns the pending rows,
//! and a paired UPDATE bumps each affected
//! `logs.messages_queue.read_index` to `GREATEST(current,
//! max_returned)` — never downgraded.

use objectiveai_sdk::cli::command::agents::logs::read::all::{
    AssistantResponsePart, AssistantResponsePartType, ClientNotificationPart,
    ClientNotificationPartType, ResponseItem, ToolResponsePart, ToolResponsePartType,
};
use sqlx::Row as _;

use super::super::{Error, Pool};
use super::row::MessageTable;

/// One materialized `logs.messages` row, with just the metadata
/// the block coalescer needs.
struct MsgRow {
    /// `logs.messages."index"` — pass to `agents logs read id`
    /// for the full typed payload.
    id: i64,
    response_id: String,
    table_kind: MessageTable,
    agent_instance_hierarchy: String,
    timestamp: i64,
}

/// Coarse block-class for a `logs.message_table` value. Block
/// boundaries are drawn whenever this changes between consecutive
/// rows (or AIH / response_id changes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockClass {
    AgentCompletionRequest,
    VectorCompletionRequest,
    FunctionExecutionRequest,
    ClientNotification,
    AssistantResponse,
    ToolResponse,
}

fn block_class(t: MessageTable) -> BlockClass {
    match t {
        MessageTable::AgentCompletionRequest => BlockClass::AgentCompletionRequest,
        MessageTable::VectorCompletionRequest => BlockClass::VectorCompletionRequest,
        MessageTable::FunctionExecutionRequest => BlockClass::FunctionExecutionRequest,
        MessageTable::MessageQueueText
        | MessageTable::MessageQueueImage
        | MessageTable::MessageQueueAudio
        | MessageTable::MessageQueueVideo
        | MessageTable::MessageQueueFile => BlockClass::ClientNotification,
        MessageTable::ToolResponse
        | MessageTable::ToolResponseContentText
        | MessageTable::ToolResponseContentImage
        | MessageTable::ToolResponseContentAudio
        | MessageTable::ToolResponseContentVideo
        | MessageTable::ToolResponseContentFile => BlockClass::ToolResponse,
        MessageTable::AssistantResponseRefusal
        | MessageTable::AssistantResponseReasoning
        | MessageTable::AssistantResponseToolCalls
        | MessageTable::AssistantResponseContentText
        | MessageTable::AssistantResponseContentImage
        | MessageTable::AssistantResponseContentAudio
        | MessageTable::AssistantResponseContentVideo
        | MessageTable::AssistantResponseContentFile => BlockClass::AssistantResponse,
    }
}

fn client_notification_kind(t: MessageTable) -> Option<ClientNotificationPartType> {
    match t {
        MessageTable::MessageQueueText => Some(ClientNotificationPartType::Text),
        MessageTable::MessageQueueImage => Some(ClientNotificationPartType::Image),
        MessageTable::MessageQueueAudio => Some(ClientNotificationPartType::Audio),
        MessageTable::MessageQueueVideo => Some(ClientNotificationPartType::Video),
        MessageTable::MessageQueueFile => Some(ClientNotificationPartType::File),
        _ => None,
    }
}

fn assistant_response_kind(t: MessageTable) -> Option<AssistantResponsePartType> {
    match t {
        MessageTable::AssistantResponseRefusal => Some(AssistantResponsePartType::Refusal),
        MessageTable::AssistantResponseReasoning => Some(AssistantResponsePartType::Reasoning),
        MessageTable::AssistantResponseToolCalls => Some(AssistantResponsePartType::ToolCall),
        MessageTable::AssistantResponseContentText => Some(AssistantResponsePartType::Text),
        MessageTable::AssistantResponseContentImage => Some(AssistantResponsePartType::Image),
        MessageTable::AssistantResponseContentAudio => Some(AssistantResponsePartType::Audio),
        MessageTable::AssistantResponseContentVideo => Some(AssistantResponsePartType::Video),
        MessageTable::AssistantResponseContentFile => Some(AssistantResponsePartType::File),
        _ => None,
    }
}

fn tool_response_kind(t: MessageTable) -> Option<ToolResponsePartType> {
    match t {
        MessageTable::ToolResponse => Some(ToolResponsePartType::Container),
        MessageTable::ToolResponseContentText => Some(ToolResponsePartType::Text),
        MessageTable::ToolResponseContentImage => Some(ToolResponsePartType::Image),
        MessageTable::ToolResponseContentAudio => Some(ToolResponsePartType::Audio),
        MessageTable::ToolResponseContentVideo => Some(ToolResponsePartType::Video),
        MessageTable::ToolResponseContentFile => Some(ToolResponsePartType::File),
        _ => None,
    }
}

/// Walk `rows` (already sorted by `id` ASC) and coalesce into
/// `ResponseItem`s. Pure / deterministic — every test smoke runs
/// through here.
fn coalesce_into_blocks(rows: Vec<MsgRow>) -> Vec<ResponseItem> {
    let mut out: Vec<ResponseItem> = Vec::new();
    let mut cur_class: Option<BlockClass> = None;
    let mut cur_aih: String = String::new();
    let mut cur_rid: String = String::new();
    let mut cur_notification_parts: Vec<ClientNotificationPart> = Vec::new();
    let mut cur_assistant_parts: Vec<AssistantResponsePart> = Vec::new();
    let mut cur_tool_parts: Vec<ToolResponsePart> = Vec::new();

    let flush = |class: Option<BlockClass>,
                 aih: &mut String,
                 rid: &mut String,
                 notification_parts: &mut Vec<ClientNotificationPart>,
                 assistant_parts: &mut Vec<AssistantResponsePart>,
                 tool_parts: &mut Vec<ToolResponsePart>,
                 out: &mut Vec<ResponseItem>| {
        match class {
            Some(BlockClass::ClientNotification) if !notification_parts.is_empty() => {
                out.push(ResponseItem::ClientNotification {
                    agent_instance_hierarchy: std::mem::take(aih),
                    response_id: std::mem::take(rid),
                    parts: std::mem::take(notification_parts),
                });
            }
            Some(BlockClass::AssistantResponse) if !assistant_parts.is_empty() => {
                out.push(ResponseItem::AssistantResponse {
                    agent_instance_hierarchy: std::mem::take(aih),
                    response_id: std::mem::take(rid),
                    parts: std::mem::take(assistant_parts),
                });
            }
            Some(BlockClass::ToolResponse) if !tool_parts.is_empty() => {
                out.push(ResponseItem::ToolResponse {
                    agent_instance_hierarchy: std::mem::take(aih),
                    response_id: std::mem::take(rid),
                    parts: std::mem::take(tool_parts),
                });
            }
            _ => {
                aih.clear();
                rid.clear();
                notification_parts.clear();
                assistant_parts.clear();
                tool_parts.clear();
            }
        }
    };

    for row in rows {
        let class = block_class(row.table_kind);

        // The three single-row request classes flush whatever's
        // pending and emit themselves as standalone items.
        match class {
            BlockClass::AgentCompletionRequest => {
                flush(
                    cur_class,
                    &mut cur_aih,
                    &mut cur_rid,
                    &mut cur_notification_parts,
                    &mut cur_assistant_parts,
                    &mut cur_tool_parts,
                    &mut out,
                );
                out.push(ResponseItem::AgentCompletionRequest {
                    id: row.id,
                    agent_instance_hierarchy: row.agent_instance_hierarchy,
                    timestamp: row.timestamp,
                    response_id: row.response_id,
                });
                cur_class = None;
                continue;
            }
            BlockClass::VectorCompletionRequest => {
                flush(
                    cur_class,
                    &mut cur_aih,
                    &mut cur_rid,
                    &mut cur_notification_parts,
                    &mut cur_assistant_parts,
                    &mut cur_tool_parts,
                    &mut out,
                );
                out.push(ResponseItem::VectorCompletionRequest {
                    id: row.id,
                    agent_instance_hierarchy: row.agent_instance_hierarchy,
                    timestamp: row.timestamp,
                    response_id: row.response_id,
                });
                cur_class = None;
                continue;
            }
            BlockClass::FunctionExecutionRequest => {
                flush(
                    cur_class,
                    &mut cur_aih,
                    &mut cur_rid,
                    &mut cur_notification_parts,
                    &mut cur_assistant_parts,
                    &mut cur_tool_parts,
                    &mut out,
                );
                out.push(ResponseItem::FunctionExecutionRequest {
                    id: row.id,
                    agent_instance_hierarchy: row.agent_instance_hierarchy,
                    timestamp: row.timestamp,
                    response_id: row.response_id,
                });
                cur_class = None;
                continue;
            }
            _ => {}
        }

        // Multi-row class. Flush if any of (class, AIH, response_id)
        // changed since the open block.
        let boundary = cur_class != Some(class)
            || cur_aih != row.agent_instance_hierarchy
            || cur_rid != row.response_id;
        if boundary {
            flush(
                cur_class,
                &mut cur_aih,
                &mut cur_rid,
                &mut cur_notification_parts,
                &mut cur_assistant_parts,
                &mut cur_tool_parts,
                &mut out,
            );
            cur_class = Some(class);
            cur_aih = row.agent_instance_hierarchy.clone();
            cur_rid = row.response_id.clone();
        }

        match class {
            BlockClass::ClientNotification => {
                let r#type = client_notification_kind(row.table_kind)
                    .expect("class invariant: ClientNotification maps to message_queue_*");
                cur_notification_parts.push(ClientNotificationPart {
                    id: row.id,
                    timestamp: row.timestamp,
                    r#type,
                });
            }
            BlockClass::AssistantResponse => {
                let r#type = assistant_response_kind(row.table_kind)
                    .expect("class invariant: AssistantResponse maps to assistant_response_*");
                cur_assistant_parts.push(AssistantResponsePart {
                    id: row.id,
                    timestamp: row.timestamp,
                    r#type,
                });
            }
            BlockClass::ToolResponse => {
                let r#type = tool_response_kind(row.table_kind)
                    .expect("class invariant: ToolResponse maps to tool_response*");
                cur_tool_parts.push(ToolResponsePart {
                    id: row.id,
                    timestamp: row.timestamp,
                    r#type,
                });
            }
            _ => unreachable!("request classes handled above"),
        }
    }

    flush(
        cur_class,
        &mut cur_aih,
        &mut cur_rid,
        &mut cur_notification_parts,
        &mut cur_assistant_parts,
        &mut cur_tool_parts,
        &mut out,
    );

    out
}

/// Materialize every `logs.messages` row for `agent_instance_hierarchy`
/// (filtered by `after_id` / `limit`), coalesced into `ResponseItem`
/// blocks.
pub async fn read_all_for_hierarchy(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    after_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<ResponseItem>, Error> {
    let rows = sqlx::query(
        "SELECT m.\"index\" AS id, m.response_id, m.\"table\" AS table_kind, \
                m.agent_instance_hierarchy, m.\"timestamp\" \
         FROM logs.messages m \
         WHERE m.agent_instance_hierarchy = $1 \
           AND m.\"index\" > COALESCE($2, 0) \
         ORDER BY m.\"index\" ASC \
         LIMIT COALESCE($3, 1000)",
    )
    .bind(agent_instance_hierarchy)
    .bind(after_id)
    .bind(limit)
    .fetch_all(&**pool)
    .await?;

    let msg_rows: Vec<MsgRow> = rows
        .into_iter()
        .map(|r| {
            Ok::<MsgRow, Error>(MsgRow {
                id: r.try_get("id")?,
                response_id: r.try_get("response_id")?,
                table_kind: r.try_get("table_kind")?,
                agent_instance_hierarchy: r.try_get("agent_instance_hierarchy")?,
                timestamp: r.try_get("timestamp")?,
            })
        })
        .collect::<Result<_, _>>()?;

    Ok(coalesce_into_blocks(msg_rows))
}

/// Materialize every unread `logs.messages` row for the children
/// spawned by `parent_agent_instance_hierarchy` (per
/// `logs.messages_queue` watermarks), coalesced into `ResponseItem`
/// blocks. Bumps each affected child's `read_index` to
/// `GREATEST(current, max_returned)` atomically in the same SQL
/// statement.
pub async fn read_pending_for_parent(
    pool: &Pool,
    parent_agent_instance_hierarchy: &str,
    after_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<ResponseItem>, Error> {
    // CTE-chained read-and-bump:
    //   * `selected` — the rows to return (filter against
    //     pre-statement `q.read_index`, plus optional caller-side
    //     `after_id` floor).
    //   * `maxes` — per-spawned max returned `id`.
    //   * `bump` — UPDATE that lifts each child's `read_index` to
    //     `GREATEST(current, max_id)`. Always runs (Postgres
    //     materializes data-modifying CTEs even when the outer
    //     SELECT doesn't reference them); when `selected` is
    //     empty, `maxes` is empty and `bump` no-ops.
    //   * Final SELECT pulls from `selected` so the caller gets
    //     the rows.
    let rows = sqlx::query(
        "WITH selected AS ( \
             SELECT m.\"index\" AS id, m.response_id, m.\"table\" AS table_kind, \
                    m.agent_instance_hierarchy, m.\"timestamp\" \
             FROM logs.messages m \
             JOIN logs.messages_queue q \
               ON q.spawned_agent_instance_hierarchy = m.agent_instance_hierarchy \
             WHERE q.parent_agent_instance_hierarchy = $1 \
               AND m.\"index\" > GREATEST(q.read_index, COALESCE($2, 0)) \
             ORDER BY m.\"index\" ASC \
             LIMIT COALESCE($3, 1000) \
         ), \
         maxes AS ( \
             SELECT agent_instance_hierarchy AS spawned, MAX(id) AS max_id \
               FROM selected \
              GROUP BY agent_instance_hierarchy \
         ), \
         bump AS ( \
             UPDATE logs.messages_queue q \
                SET read_index = GREATEST(q.read_index, m.max_id) \
               FROM maxes m \
              WHERE q.parent_agent_instance_hierarchy = $1 \
                AND q.spawned_agent_instance_hierarchy = m.spawned \
             RETURNING 1 \
         ) \
         SELECT s.id, s.response_id, s.table_kind, \
                s.agent_instance_hierarchy, s.\"timestamp\" \
           FROM selected s \
          ORDER BY s.id ASC",
    )
    .bind(parent_agent_instance_hierarchy)
    .bind(after_id)
    .bind(limit)
    .fetch_all(&**pool)
    .await?;

    let msg_rows: Vec<MsgRow> = rows
        .into_iter()
        .map(|r| {
            Ok::<MsgRow, Error>(MsgRow {
                id: r.try_get("id")?,
                response_id: r.try_get("response_id")?,
                table_kind: r.try_get("table_kind")?,
                agent_instance_hierarchy: r.try_get("agent_instance_hierarchy")?,
                timestamp: r.try_get("timestamp")?,
            })
        })
        .collect::<Result<_, _>>()?;

    Ok(coalesce_into_blocks(msg_rows))
}
