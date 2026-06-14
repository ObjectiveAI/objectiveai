//! `agents logs read all` / `agents logs read pending` backend:
//! SELECT `objectiveai.messages` rows for a target AIH (or every child
//! AIH of a parent), JOIN through to the row's source table to
//! pull `sender_agent_instance_hierarchy` (+ `timestamp_queued`
//! for `message_queue_*` kinds), coalesce consecutive rows into
//! `ResponseItem` blocks, and yield them in index order.
//!
//! Sender + timestamp_queued live on the row's source table — the
//! three `logs.<tier>_completion_requests` tables for request +
//! assistant_response_* + tool_response* rows (all reachable by
//! `response_id`), and `message_queue` via
//! `message_queue_contents` for the five `message_queue_*` row
//! kinds. We LEFT JOIN all four sources unconditionally and let
//! the `CASE` over `m."table"` pick the right column. No
//! denormalized shadow copies on `objectiveai.messages`.
//!
//! Block-coalesce rule: a new block starts when ANY of `(class,
//! agent_instance_hierarchy, response_id)` changes — PLUS, for
//! `ClientNotification` rows, when the `sender_agent_instance_hierarchy`
//! changes. Assistant/Tool blocks ignore sender because their
//! producer IS the agent (no separate sender exists). The three
//! request-blob classes are always single-row blocks.
//!
//! `read pending` is read-and-advance, expressed as a single
//! CTE-chained SQL statement: the SELECT returns the pending rows,
//! and a paired UPDATE bumps each affected
//! `objectiveai.messages_queue.read_index` to `GREATEST(current,
//! max_returned)` — never downgraded.

use objectiveai_sdk::cli::command::agents::logs::read::all::{
    AssistantResponsePart, ClientNotificationPart, ClientNotificationPartType, ResponseItem,
    ToolResponsePart, ToolResponsePartType,
};
use sqlx::Row as _;

use super::super::{Error, Pool};
use super::row::MessageTable;

/// One materialized `objectiveai.messages` row plus the joined-in sender
/// (and queue parent + enqueued_at for `message_queue_*` rows).
struct MsgRow {
    /// `objectiveai.messages."index"` — pass to `agents logs read id`
    /// for the full typed payload.
    id: i64,
    response_id: String,
    table_kind: MessageTable,
    agent_instance_hierarchy: String,
    timestamp_delivered: i64,
    /// Sender AIH. Populated for request blob rows (from
    /// `logs.<tier>_completion_requests.sender_*`) and for
    /// `message_queue_*` rows (from `message_queue.sender_*`).
    /// NULL for assistant/tool response rows — those have no
    /// distinct sender, the agent IS the producer.
    sender_agent_instance_hierarchy: Option<String>,
    /// `message_queue.id` of the consumed parent queue row.
    /// Some only for `message_queue_*` rows. Part of the
    /// `ClientNotification` block boundary tuple so each block
    /// = exactly one parent queue row.
    message_queue_id: Option<i64>,
    /// `message_queue.enqueued_at` of the consumed parent queue
    /// row. Some only for `message_queue_*` rows; lives at
    /// block level on the emitted `ClientNotification`.
    timestamp_queued: Option<i64>,
    /// `message_queue.key` of the consumed parent queue row —
    /// the idempotency token passed to
    /// `agents message --enqueue-with-key`. Some only for
    /// `message_queue_*` rows, and only when the parent row had
    /// a key set; lives at block level on the emitted
    /// `ClientNotification`.
    message_queue_key: Option<String>,
    /// `objectiveai.assistant_response_tool_calls.function_name` —
    /// `Some` only for tool-call rows; NULL → `None` for every other
    /// table. Surfaced on [`AssistantResponsePart::ToolCall`] so
    /// callers can dedupe tool calls by name without a round-trip
    /// through `agents logs read id`.
    function_name: Option<String>,
    /// `tool_call_id` for the row — `COALESCE` of the
    /// `assistant_response_tool_calls` join (assistant tool-call
    /// rows) and the `tool_response` join (tool-response content
    /// rows); these are mutually exclusive per row. `None` for every
    /// other table. Used both to inline it on
    /// `AssistantResponsePart::ToolCall` and to split `ToolResponse`
    /// blocks per tool call.
    tool_call_id: Option<String>,
    /// `objectiveai.messages.row_sub_index` — the tool call's wire
    /// index for `assistant_response_tool_calls` rows (and the
    /// part_index for content rows). Surfaced as `tool_call_index` on
    /// `AssistantResponsePart::ToolCall`.
    row_sub_index: Option<i64>,
}

/// Coarse block-class for a `objectiveai.message_table` value. Block
/// boundaries are drawn whenever this changes between consecutive
/// rows (or AIH / response_id / sender for ClientNotification).
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
        MessageTable::ToolResponseContentText
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

/// Build the [`AssistantResponsePart`] for one `assistant_response_*`
/// row. The `ToolCall` variant inlines the call's metadata
/// (`function_name` / `tool_call_id` / `tool_call_index`); every other
/// variant carries just `id` + `timestamp_delivered`.
fn assistant_response_part(row: &MsgRow) -> Option<AssistantResponsePart> {
    let id = row.id;
    let timestamp_delivered = row.timestamp_delivered;
    Some(match row.table_kind {
        MessageTable::AssistantResponseToolCalls => AssistantResponsePart::ToolCall {
            id,
            timestamp_delivered,
            function_name: row.function_name.clone().unwrap_or_default(),
            tool_call_id: row.tool_call_id.clone().unwrap_or_default(),
            tool_call_index: row.row_sub_index.unwrap_or_default(),
        },
        MessageTable::AssistantResponseRefusal => {
            AssistantResponsePart::Refusal { id, timestamp_delivered }
        }
        MessageTable::AssistantResponseReasoning => {
            AssistantResponsePart::Reasoning { id, timestamp_delivered }
        }
        MessageTable::AssistantResponseContentText => {
            AssistantResponsePart::Text { id, timestamp_delivered }
        }
        MessageTable::AssistantResponseContentImage => {
            AssistantResponsePart::Image { id, timestamp_delivered }
        }
        MessageTable::AssistantResponseContentAudio => {
            AssistantResponsePart::Audio { id, timestamp_delivered }
        }
        MessageTable::AssistantResponseContentVideo => {
            AssistantResponsePart::Video { id, timestamp_delivered }
        }
        MessageTable::AssistantResponseContentFile => {
            AssistantResponsePart::File { id, timestamp_delivered }
        }
        _ => return None,
    })
}

fn tool_response_kind(t: MessageTable) -> Option<ToolResponsePartType> {
    match t {
        MessageTable::ToolResponseContentText => Some(ToolResponsePartType::Text),
        MessageTable::ToolResponseContentImage => Some(ToolResponsePartType::Image),
        MessageTable::ToolResponseContentAudio => Some(ToolResponsePartType::Audio),
        MessageTable::ToolResponseContentVideo => Some(ToolResponsePartType::Video),
        MessageTable::ToolResponseContentFile => Some(ToolResponsePartType::File),
        _ => None,
    }
}

/// Shared SELECT clause for `read_all` / `read_pending`. JOINs
/// the four sender source tables LEFT-style; CASE-picks the
/// right sender column based on `m."table"`. `timestamp_queued`
/// comes from the queue JOIN (Some only for `message_queue_*`
/// kinds).
const SELECT_SHAPE: &str = "SELECT \
    m.\"index\" AS id, \
    m.response_id, \
    m.\"table\" AS table_kind, \
    m.agent_instance_hierarchy, \
    m.\"timestamp\" AS timestamp_delivered, \
    CASE m.\"table\" \
        WHEN 'message_queue_text'  THEN mq.sender_agent_instance_hierarchy \
        WHEN 'message_queue_image' THEN mq.sender_agent_instance_hierarchy \
        WHEN 'message_queue_audio' THEN mq.sender_agent_instance_hierarchy \
        WHEN 'message_queue_video' THEN mq.sender_agent_instance_hierarchy \
        WHEN 'message_queue_file'  THEN mq.sender_agent_instance_hierarchy \
        WHEN 'agent_completion_request'    THEN acr.sender_agent_instance_hierarchy \
        WHEN 'vector_completion_request'   THEN vcr.sender_agent_instance_hierarchy \
        WHEN 'function_execution_request'  THEN fer.sender_agent_instance_hierarchy \
        ELSE NULL \
    END AS sender_agent_instance_hierarchy, \
    mq.id AS message_queue_id, \
    mq.enqueued_at AS timestamp_queued, \
    mq.key AS message_queue_key, \
    atc.function_name AS function_name, \
    COALESCE(atc.tool_call_id, tr.tool_call_id) AS tool_call_id, \
    m.row_sub_index AS row_sub_index";

const FROM_JOINS: &str = "FROM objectiveai.messages m \
    LEFT JOIN objectiveai.message_queue_contents mqc \
        ON m.row_index = mqc.id \
        AND m.\"table\" IN ( \
            'message_queue_text', 'message_queue_image', 'message_queue_audio', \
            'message_queue_video', 'message_queue_file' \
        ) \
    LEFT JOIN objectiveai.message_queue mq ON mqc.message_queue_id = mq.id \
    LEFT JOIN objectiveai.agent_completion_requests acr \
        ON m.response_id = acr.response_id \
        AND m.\"table\" = 'agent_completion_request' \
    LEFT JOIN objectiveai.vector_completion_requests vcr \
        ON m.response_id = vcr.response_id \
        AND m.\"table\" = 'vector_completion_request' \
    LEFT JOIN objectiveai.function_execution_requests fer \
        ON m.response_id = fer.response_id \
        AND m.\"table\" = 'function_execution_request' \
    LEFT JOIN objectiveai.assistant_response_tool_calls atc \
        ON m.response_id = atc.response_id \
        AND m.row_index = atc.\"index\" \
        AND m.row_sub_index = atc.tool_call_index \
        AND m.\"table\" = 'assistant_response_tool_calls' \
    LEFT JOIN objectiveai.tool_response tr \
        ON m.response_id = tr.response_id \
        AND m.row_index = tr.\"index\" \
        AND m.\"table\" IN ( \
            'tool_response_content_text', 'tool_response_content_image', \
            'tool_response_content_audio', 'tool_response_content_video', \
            'tool_response_content_file' \
        )";

fn row_into_msg(r: &sqlx::postgres::PgRow) -> Result<MsgRow, Error> {
    Ok(MsgRow {
        id: r.try_get("id")?,
        response_id: r.try_get("response_id")?,
        table_kind: r.try_get("table_kind")?,
        agent_instance_hierarchy: r.try_get("agent_instance_hierarchy")?,
        timestamp_delivered: r.try_get("timestamp_delivered")?,
        sender_agent_instance_hierarchy: r.try_get("sender_agent_instance_hierarchy")?,
        message_queue_id: r.try_get("message_queue_id")?,
        timestamp_queued: r.try_get("timestamp_queued")?,
        message_queue_key: r.try_get("message_queue_key")?,
        function_name: r.try_get("function_name")?,
        tool_call_id: r.try_get("tool_call_id")?,
        row_sub_index: r.try_get("row_sub_index")?,
    })
}

/// Walk `rows` (already sorted by `id` ASC) and coalesce into
/// `ResponseItem`s. Pure / deterministic.
fn coalesce_into_blocks(rows: Vec<MsgRow>) -> Vec<ResponseItem> {
    let mut out: Vec<ResponseItem> = Vec::new();
    let mut cur_class: Option<BlockClass> = None;
    let mut cur_aih: String = String::new();
    let mut cur_rid: String = String::new();
    /// `Some` only for an open `ClientNotification` block; assistant /
    /// tool blocks never set this. Boundary check pulls it in.
    let mut cur_sender: Option<String> = None;
    /// `Some` only for an open `ClientNotification` block — the
    /// consumed `message_queue.id`. Forces 1:1 block-to-parent
    /// correspondence so block-level `timestamp_queued` is
    /// well-defined.
    let mut cur_mq_id: Option<i64> = None;
    /// `Some` only for an open `ClientNotification` block —
    /// `message_queue.enqueued_at`.
    let mut cur_timestamp_queued: Option<i64> = None;
    /// `Some` only for an open `ClientNotification` block AND
    /// only when the parent queue row had `--key` set —
    /// `message_queue.key`.
    let mut cur_key: Option<String> = None;
    // `Some` only for an open `ToolResponse` block — the `tool_call_id`
    // this block answers. Part of the ToolResponse boundary tuple so
    // each block = exactly one tool call's response.
    let mut cur_tool_call_id: Option<String> = None;
    let mut cur_notification_parts: Vec<ClientNotificationPart> = Vec::new();
    let mut cur_assistant_parts: Vec<AssistantResponsePart> = Vec::new();
    let mut cur_tool_parts: Vec<ToolResponsePart> = Vec::new();

    let flush = |class: Option<BlockClass>,
                 aih: &mut String,
                 rid: &mut String,
                 sender: &mut Option<String>,
                 mq_id: &mut Option<i64>,
                 timestamp_queued: &mut Option<i64>,
                 key: &mut Option<String>,
                 tool_call_id: &mut Option<String>,
                 notification_parts: &mut Vec<ClientNotificationPart>,
                 assistant_parts: &mut Vec<AssistantResponsePart>,
                 tool_parts: &mut Vec<ToolResponsePart>,
                 out: &mut Vec<ResponseItem>| {
        match class {
            Some(BlockClass::ClientNotification) if !notification_parts.is_empty() => {
                out.push(ResponseItem::ClientNotification {
                    agent_instance_hierarchy: std::mem::take(aih),
                    sender_agent_instance_hierarchy: sender.take().unwrap_or_default(),
                    response_id: std::mem::take(rid),
                    timestamp_queued: timestamp_queued.take().unwrap_or_default(),
                    key: key.take(),
                    parts: std::mem::take(notification_parts),
                });
                *mq_id = None;
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
                    tool_call_id: tool_call_id.take().unwrap_or_default(),
                    parts: std::mem::take(tool_parts),
                });
            }
            _ => {
                aih.clear();
                rid.clear();
                *sender = None;
                *mq_id = None;
                *timestamp_queued = None;
                *key = None;
                *tool_call_id = None;
                notification_parts.clear();
                assistant_parts.clear();
                tool_parts.clear();
            }
        }
    };

    for row in rows {
        let class = block_class(row.table_kind);

        // Single-row request classes — emit immediately, reset.
        match class {
            BlockClass::AgentCompletionRequest => {
                flush(
                    cur_class, &mut cur_aih, &mut cur_rid, &mut cur_sender,
                    &mut cur_mq_id, &mut cur_timestamp_queued, &mut cur_key,
                    &mut cur_tool_call_id,
                    &mut cur_notification_parts, &mut cur_assistant_parts,
                    &mut cur_tool_parts, &mut out,
                );
                out.push(ResponseItem::AgentCompletionRequest {
                    id: row.id,
                    agent_instance_hierarchy: row.agent_instance_hierarchy,
                    sender_agent_instance_hierarchy: row
                        .sender_agent_instance_hierarchy
                        .unwrap_or_default(),
                    timestamp_delivered: row.timestamp_delivered,
                    response_id: row.response_id,
                });
                cur_class = None;
                continue;
            }
            BlockClass::VectorCompletionRequest => {
                flush(
                    cur_class, &mut cur_aih, &mut cur_rid, &mut cur_sender,
                    &mut cur_mq_id, &mut cur_timestamp_queued, &mut cur_key,
                    &mut cur_tool_call_id,
                    &mut cur_notification_parts, &mut cur_assistant_parts,
                    &mut cur_tool_parts, &mut out,
                );
                out.push(ResponseItem::VectorCompletionRequest {
                    id: row.id,
                    agent_instance_hierarchy: row.agent_instance_hierarchy,
                    sender_agent_instance_hierarchy: row
                        .sender_agent_instance_hierarchy
                        .unwrap_or_default(),
                    timestamp_delivered: row.timestamp_delivered,
                    response_id: row.response_id,
                });
                cur_class = None;
                continue;
            }
            BlockClass::FunctionExecutionRequest => {
                flush(
                    cur_class, &mut cur_aih, &mut cur_rid, &mut cur_sender,
                    &mut cur_mq_id, &mut cur_timestamp_queued, &mut cur_key,
                    &mut cur_tool_call_id,
                    &mut cur_notification_parts, &mut cur_assistant_parts,
                    &mut cur_tool_parts, &mut out,
                );
                out.push(ResponseItem::FunctionExecutionRequest {
                    id: row.id,
                    agent_instance_hierarchy: row.agent_instance_hierarchy,
                    sender_agent_instance_hierarchy: row
                        .sender_agent_instance_hierarchy
                        .unwrap_or_default(),
                    timestamp_delivered: row.timestamp_delivered,
                    response_id: row.response_id,
                });
                cur_class = None;
                continue;
            }
            _ => {}
        }

        // Multi-row class. For ClientNotification, sender_aih
        // AND message_queue_id are part of the boundary tuple —
        // each block = one consumed parent queue row, well-defined
        // block-level `timestamp_queued`. Assistant/Tool blocks
        // ignore sender + mq_id (both are None for them anyway).
        let boundary = cur_class != Some(class)
            || cur_aih != row.agent_instance_hierarchy
            || cur_rid != row.response_id
            || (class == BlockClass::ClientNotification
                && (cur_sender.as_deref() != row.sender_agent_instance_hierarchy.as_deref()
                    || cur_mq_id != row.message_queue_id))
            || (class == BlockClass::ToolResponse
                && cur_tool_call_id.as_deref() != row.tool_call_id.as_deref());
        if boundary {
            flush(
                cur_class, &mut cur_aih, &mut cur_rid, &mut cur_sender,
                &mut cur_mq_id, &mut cur_timestamp_queued, &mut cur_key,
                &mut cur_tool_call_id,
                &mut cur_notification_parts, &mut cur_assistant_parts,
                &mut cur_tool_parts, &mut out,
            );
            cur_class = Some(class);
            cur_aih = row.agent_instance_hierarchy.clone();
            cur_rid = row.response_id.clone();
            if class == BlockClass::ClientNotification {
                cur_sender = row.sender_agent_instance_hierarchy.clone();
                cur_mq_id = row.message_queue_id;
                cur_timestamp_queued = row.timestamp_queued;
                cur_key = row.message_queue_key.clone();
                cur_tool_call_id = None;
            } else if class == BlockClass::ToolResponse {
                cur_sender = None;
                cur_mq_id = None;
                cur_timestamp_queued = None;
                cur_key = None;
                cur_tool_call_id = row.tool_call_id.clone();
            } else {
                cur_sender = None;
                cur_mq_id = None;
                cur_timestamp_queued = None;
                cur_key = None;
                cur_tool_call_id = None;
            }
        }

        match class {
            BlockClass::ClientNotification => {
                let r#type = client_notification_kind(row.table_kind)
                    .expect("class invariant: ClientNotification maps to message_queue_*");
                cur_notification_parts.push(ClientNotificationPart {
                    id: row.id,
                    timestamp_delivered: row.timestamp_delivered,
                    r#type,
                });
            }
            BlockClass::AssistantResponse => {
                let part = assistant_response_part(&row)
                    .expect("class invariant: AssistantResponse maps to assistant_response_*");
                cur_assistant_parts.push(part);
            }
            BlockClass::ToolResponse => {
                let r#type = tool_response_kind(row.table_kind)
                    .expect("class invariant: ToolResponse maps to tool_response*");
                cur_tool_parts.push(ToolResponsePart {
                    id: row.id,
                    timestamp_delivered: row.timestamp_delivered,
                    r#type,
                });
            }
            _ => unreachable!("request classes handled above"),
        }
    }

    flush(
        cur_class, &mut cur_aih, &mut cur_rid, &mut cur_sender,
        &mut cur_mq_id, &mut cur_timestamp_queued, &mut cur_key,
        &mut cur_tool_call_id,
        &mut cur_notification_parts, &mut cur_assistant_parts,
        &mut cur_tool_parts, &mut out,
    );

    out
}

/// Materialize every `objectiveai.messages` row for `agent_instance_hierarchy`
/// (filtered by `after_id` / `limit`), coalesced into `ResponseItem`
/// blocks.
pub async fn read_all_for_hierarchy(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    after_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<ResponseItem>, Error> {
    let sql = format!(
        "{select} {from} \
         WHERE m.agent_instance_hierarchy = $1 \
           AND m.\"index\" > COALESCE($2, 0) \
         ORDER BY m.\"index\" ASC \
         LIMIT $3",
        select = SELECT_SHAPE,
        from = FROM_JOINS,
    );
    let rows = sqlx::query(&sql)
        .bind(agent_instance_hierarchy)
        .bind(after_id)
        .bind(limit)
        .fetch_all(&**pool)
        .await?;

    let msg_rows: Vec<MsgRow> = rows.iter().map(row_into_msg).collect::<Result<_, _>>()?;
    Ok(coalesce_into_blocks(msg_rows))
}

/// Materialize every unread `objectiveai.messages` row for the children
/// spawned by `parent_agent_instance_hierarchy` (per
/// `objectiveai.messages_queue` watermarks), coalesced into `ResponseItem`
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
    //   * `selected` — the rows to return; same JOIN topology as
    //     `read_all_for_hierarchy` plus a JOIN to
    //     `objectiveai.messages_queue` for the watermark filter.
    //   * `maxes` — per-spawned max returned `id`.
    //   * `bump` — UPDATE that lifts each child's `read_index` to
    //     `GREATEST(current, max_id)`. Always runs (Postgres
    //     materializes data-modifying CTEs even when the outer
    //     SELECT doesn't reference them); when `selected` is
    //     empty, `maxes` is empty and `bump` no-ops.
    //   * Final SELECT pulls from `selected`.
    let sql = format!(
        "WITH selected AS ( \
             {select} \
             {from} \
             JOIN objectiveai.messages_queue q \
               ON q.spawned_agent_instance_hierarchy = m.agent_instance_hierarchy \
             WHERE q.parent_agent_instance_hierarchy = $1 \
               AND m.\"index\" > GREATEST(q.read_index, COALESCE($2, 0)) \
             ORDER BY m.\"index\" ASC \
             LIMIT $3 \
         ), \
         maxes AS ( \
             SELECT agent_instance_hierarchy AS spawned, MAX(id) AS max_id \
               FROM selected \
              GROUP BY agent_instance_hierarchy \
         ), \
         bump AS ( \
             UPDATE objectiveai.messages_queue qq \
                SET read_index = GREATEST(qq.read_index, mx.max_id) \
               FROM maxes mx \
              WHERE qq.parent_agent_instance_hierarchy = $1 \
                AND qq.spawned_agent_instance_hierarchy = mx.spawned \
             RETURNING 1 \
         ) \
         SELECT s.id, s.response_id, s.table_kind, \
                s.agent_instance_hierarchy, s.timestamp_delivered, \
                s.sender_agent_instance_hierarchy, \
                s.message_queue_id, s.timestamp_queued, \
                s.message_queue_key, s.function_name, \
                s.tool_call_id, s.row_sub_index \
           FROM selected s \
          ORDER BY s.id ASC",
        select = SELECT_SHAPE,
        from = FROM_JOINS,
    );
    let rows = sqlx::query(&sql)
        .bind(parent_agent_instance_hierarchy)
        .bind(after_id)
        .bind(limit)
        .fetch_all(&**pool)
        .await?;

    let msg_rows: Vec<MsgRow> = rows.iter().map(row_into_msg).collect::<Result<_, _>>()?;
    Ok(coalesce_into_blocks(msg_rows))
}

/// Side-effect-free existence check used by
/// `agents logs read subscribe`'s wait loop. Returns `true` iff
/// `objectiveai.messages_queue` has at least one unread row past the
/// watermark for any child of `parent_agent_instance_hierarchy`
/// whose `m."table"` falls in `kinds`. When `kinds` is `None` or
/// empty, the kind filter is dropped (existence check across all
/// kinds — equivalent to "is there anything pending at all?").
///
/// Does NOT touch `read_index`. The subscriber re-checks via
/// this on every wake-up and only calls
/// `read_pending_for_parent` (which DOES bump) once it confirms
/// a matching row exists. When a match is confirmed, the
/// subsequent `read_pending_for_parent` call returns EVERY
/// pending row regardless of kind — the kinds filter is for
/// "wake me up" gating only, not for the returned slice.
pub async fn any_pending_matching_kinds(
    pool: &Pool,
    parent_agent_instance_hierarchy: &str,
    after_id: Option<i64>,
    kinds: Option<&[MessageTable]>,
) -> Result<bool, Error> {
    let kinds_clause = match kinds {
        Some(ks) if !ks.is_empty() => {
            let list = ks
                .iter()
                .map(|k| format!("'{}'", k.schema_name()))
                .collect::<Vec<_>>()
                .join(", ");
            format!("AND m.\"table\" IN ({list})")
        }
        _ => String::new(),
    };
    let sql = format!(
        "SELECT EXISTS( \
             SELECT 1 FROM objectiveai.messages m \
             JOIN objectiveai.messages_queue q \
               ON q.spawned_agent_instance_hierarchy = m.agent_instance_hierarchy \
             WHERE q.parent_agent_instance_hierarchy = $1 \
               AND m.\"index\" > GREATEST(q.read_index, COALESCE($2, 0)) \
               {kinds_clause} \
         )"
    );
    let exists: bool = sqlx::query_scalar(&sql)
        .bind(parent_agent_instance_hierarchy)
        .bind(after_id)
        .fetch_one(&**pool)
        .await?;
    Ok(exists)
}
