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

use objectiveai_sdk::cli::command::agents::logs::list::{
    AssistantResponsePart, ClientNotificationPart, ClientNotificationPartType, ResponseItem,
    RequestMessageUserPart, RequestMessageUserPartType, ToolResponsePart, ToolResponsePartType,
    VectorRequestChoice, VectorRequestChoicePart, VectorRequestChoicePartType,
};
use sqlx::Row as _;

use super::super::time::unix_to_rfc3339;
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
    /// `objectiveai.messages.row_index` — needed to group
    /// `request_vector_choice_content_*` rows into choices by choice
    /// index. NULL for `response_vector_vote`.
    row_index: Option<i64>,
    /// `request_vector_choice.key` for the choice — `Some` only for
    /// `request_vector_choice_content_*` rows (JOINed by
    /// (response_id, choice index)). Surfaced inline on the choice.
    choice_key: Option<String>,
    /// `response_vector_vote.vote` JSON array — `Some` only for
    /// `response_vector_vote` rows. Parsed into `Vec<Decimal>` and
    /// returned inline (no `read id`).
    vote: Option<serde_json::Value>,
}

/// Coarse block-class for a `objectiveai.message_table` value. Block
/// boundaries are drawn whenever this changes between consecutive
/// rows (or AIH / response_id / sender for ClientNotification).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockClass {
    /// Request-blob marker rows — still written (blob openable) but
    /// NOT emitted in `list`. Collapsed into one skipped class.
    SkippedRequestBlob,
    ClientNotification,
    AssistantResponse,
    ToolResponse,
    RequestMessageUser,
    RequestMessageAssistant,
    RequestMessageTool,
    VectorRequestChoices,
    VectorResponseVote,
}

fn block_class(t: MessageTable) -> BlockClass {
    match t {
        MessageTable::AgentCompletionRequest
        | MessageTable::VectorCompletionRequest
        | MessageTable::FunctionExecutionRequest => BlockClass::SkippedRequestBlob,
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
        MessageTable::RequestMessageUserContentText
        | MessageTable::RequestMessageUserContentImage
        | MessageTable::RequestMessageUserContentAudio
        | MessageTable::RequestMessageUserContentVideo
        | MessageTable::RequestMessageUserContentFile => BlockClass::RequestMessageUser,
        MessageTable::RequestMessageAssistantRefusal
        | MessageTable::RequestMessageAssistantReasoning
        | MessageTable::RequestMessageAssistantToolCalls
        | MessageTable::RequestMessageAssistantContentText
        | MessageTable::RequestMessageAssistantContentImage
        | MessageTable::RequestMessageAssistantContentAudio
        | MessageTable::RequestMessageAssistantContentVideo
        | MessageTable::RequestMessageAssistantContentFile => BlockClass::RequestMessageAssistant,
        MessageTable::RequestMessageToolContentText
        | MessageTable::RequestMessageToolContentImage
        | MessageTable::RequestMessageToolContentAudio
        | MessageTable::RequestMessageToolContentVideo
        | MessageTable::RequestMessageToolContentFile => BlockClass::RequestMessageTool,
        MessageTable::RequestVectorChoiceContentText
        | MessageTable::RequestVectorChoiceContentImage
        | MessageTable::RequestVectorChoiceContentAudio
        | MessageTable::RequestVectorChoiceContentVideo
        | MessageTable::RequestVectorChoiceContentFile => BlockClass::VectorRequestChoices,
        MessageTable::ResponseVectorVote => BlockClass::VectorResponseVote,
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
/// variant carries just `id` + `delivered_at`.
fn assistant_response_part(row: &MsgRow) -> Option<AssistantResponsePart> {
    let id = row.id;
    let delivered_at = unix_to_rfc3339(row.timestamp_delivered);
    Some(match row.table_kind {
        MessageTable::AssistantResponseToolCalls
        | MessageTable::RequestMessageAssistantToolCalls => AssistantResponsePart::ToolCall {
            id,
            delivered_at,
            function_name: row.function_name.clone().unwrap_or_default(),
            tool_call_id: row.tool_call_id.clone().unwrap_or_default(),
            tool_call_index: row.row_sub_index.unwrap_or_default(),
        },
        MessageTable::AssistantResponseRefusal
        | MessageTable::RequestMessageAssistantRefusal => {
            AssistantResponsePart::Refusal { id, delivered_at }
        }
        MessageTable::AssistantResponseReasoning
        | MessageTable::RequestMessageAssistantReasoning => {
            AssistantResponsePart::Reasoning { id, delivered_at }
        }
        MessageTable::AssistantResponseContentText
        | MessageTable::RequestMessageAssistantContentText => {
            AssistantResponsePart::Text { id, delivered_at }
        }
        MessageTable::AssistantResponseContentImage
        | MessageTable::RequestMessageAssistantContentImage => {
            AssistantResponsePart::Image { id, delivered_at }
        }
        MessageTable::AssistantResponseContentAudio
        | MessageTable::RequestMessageAssistantContentAudio => {
            AssistantResponsePart::Audio { id, delivered_at }
        }
        MessageTable::AssistantResponseContentVideo
        | MessageTable::RequestMessageAssistantContentVideo => {
            AssistantResponsePart::Video { id, delivered_at }
        }
        MessageTable::AssistantResponseContentFile
        | MessageTable::RequestMessageAssistantContentFile => {
            AssistantResponsePart::File { id, delivered_at }
        }
        _ => return None,
    })
}

fn tool_response_kind(t: MessageTable) -> Option<ToolResponsePartType> {
    match t {
        MessageTable::ToolResponseContentText
        | MessageTable::RequestMessageToolContentText => Some(ToolResponsePartType::Text),
        MessageTable::ToolResponseContentImage
        | MessageTable::RequestMessageToolContentImage => Some(ToolResponsePartType::Image),
        MessageTable::ToolResponseContentAudio
        | MessageTable::RequestMessageToolContentAudio => Some(ToolResponsePartType::Audio),
        MessageTable::ToolResponseContentVideo
        | MessageTable::RequestMessageToolContentVideo => Some(ToolResponsePartType::Video),
        MessageTable::ToolResponseContentFile
        | MessageTable::RequestMessageToolContentFile => Some(ToolResponsePartType::File),
        _ => None,
    }
}

fn request_message_user_kind(t: MessageTable) -> Option<RequestMessageUserPartType> {
    match t {
        MessageTable::RequestMessageUserContentText => Some(RequestMessageUserPartType::Text),
        MessageTable::RequestMessageUserContentImage => Some(RequestMessageUserPartType::Image),
        MessageTable::RequestMessageUserContentAudio => Some(RequestMessageUserPartType::Audio),
        MessageTable::RequestMessageUserContentVideo => Some(RequestMessageUserPartType::Video),
        MessageTable::RequestMessageUserContentFile => Some(RequestMessageUserPartType::File),
        _ => None,
    }
}

fn vector_choice_kind(t: MessageTable) -> Option<VectorRequestChoicePartType> {
    match t {
        MessageTable::RequestVectorChoiceContentText => Some(VectorRequestChoicePartType::Text),
        MessageTable::RequestVectorChoiceContentImage => Some(VectorRequestChoicePartType::Image),
        MessageTable::RequestVectorChoiceContentAudio => Some(VectorRequestChoicePartType::Audio),
        MessageTable::RequestVectorChoiceContentVideo => Some(VectorRequestChoicePartType::Video),
        MessageTable::RequestVectorChoiceContentFile => Some(VectorRequestChoicePartType::File),
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
    COALESCE(atc.function_name, rmatc.function_name) AS function_name, \
    COALESCE(atc.tool_call_id, tr.tool_call_id, rmatc.tool_call_id, rmt.tool_call_id) AS tool_call_id, \
    m.row_sub_index AS row_sub_index, \
    m.row_index AS row_index, \
    rvc.key AS choice_key, \
    rvv.vote AS vote";

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
        ) \
    LEFT JOIN objectiveai.request_message_assistant_tool_calls rmatc \
        ON m.response_id = rmatc.response_id \
        AND m.row_index = rmatc.\"index\" \
        AND m.row_sub_index = rmatc.tool_call_index \
        AND m.\"table\" = 'request_message_assistant_tool_calls' \
    LEFT JOIN objectiveai.request_message_tool rmt \
        ON m.response_id = rmt.response_id \
        AND m.row_index = rmt.\"index\" \
        AND m.\"table\" IN ( \
            'request_message_tool_content_text', 'request_message_tool_content_image', \
            'request_message_tool_content_audio', 'request_message_tool_content_video', \
            'request_message_tool_content_file' \
        ) \
    LEFT JOIN objectiveai.request_vector_choice rvc \
        ON m.response_id = rvc.response_id \
        AND m.row_index = rvc.\"index\" \
        AND m.\"table\" IN ( \
            'request_vector_choice_content_text', 'request_vector_choice_content_image', \
            'request_vector_choice_content_audio', 'request_vector_choice_content_video', \
            'request_vector_choice_content_file' \
        ) \
    LEFT JOIN objectiveai.response_vector_vote rvv \
        ON m.response_id = rvv.response_id \
        AND m.agent_instance_hierarchy = rvv.agent_instance_hierarchy \
        AND m.\"table\" = 'response_vector_vote'";

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
        row_index: r.try_get("row_index")?,
        choice_key: r.try_get("choice_key")?,
        vote: r.try_get::<Option<sqlx::types::Json<serde_json::Value>>, _>("vote")?.map(|j| j.0),
    })
}

/// Accumulation state for one in-progress `ResponseItem` block while
/// walking `objectiveai.messages` rows in `"index"` order.
#[derive(Default)]
struct BlockAccum {
    class: Option<BlockClass>,
    aih: String,
    rid: String,
    sender: Option<String>,
    mq_id: Option<i64>,
    timestamp_queued: Option<i64>,
    key: Option<String>,
    /// The tool_call_id for an open ToolResponse / RequestMessageTool
    /// block (part of their boundary tuple).
    tool_call_id: Option<String>,
    notification_parts: Vec<ClientNotificationPart>,
    /// Shared by AssistantResponse and RequestMessageAssistant blocks
    /// (same part shape; the open `class` selects the emitted variant).
    assistant_parts: Vec<AssistantResponsePart>,
    /// Shared by ToolResponse and RequestMessageTool blocks.
    tool_parts: Vec<ToolResponsePart>,
    user_parts: Vec<RequestMessageUserPart>,
    // VectorRequestChoices: choices grouped by choice index (row_index).
    choices: Vec<VectorRequestChoice>,
    cur_choice_index: Option<i64>,
    cur_choice_key: Option<String>,
    cur_choice_parts: Vec<VectorRequestChoicePart>,
}

impl BlockAccum {
    /// Push the in-progress choice (if any) into `choices`.
    fn finish_choice(&mut self) {
        if self.cur_choice_index.is_some() || !self.cur_choice_parts.is_empty() {
            self.choices.push(VectorRequestChoice {
                key: self.cur_choice_key.take().unwrap_or_default(),
                parts: std::mem::take(&mut self.cur_choice_parts),
            });
        }
        self.cur_choice_index = None;
    }

    /// Emit the open block (if it has content), then reset all state.
    fn flush(&mut self, out: &mut Vec<ResponseItem>) {
        match self.class {
            Some(BlockClass::ClientNotification) if !self.notification_parts.is_empty() => {
                out.push(ResponseItem::ClientNotification {
                    agent_instance_hierarchy: std::mem::take(&mut self.aih),
                    sender_agent_instance_hierarchy: self.sender.take().unwrap_or_default(),
                    response_id: std::mem::take(&mut self.rid),
                    queued_at: unix_to_rfc3339(self.timestamp_queued.take().unwrap_or_default()),
                    key: self.key.take(),
                    parts: std::mem::take(&mut self.notification_parts),
                });
            }
            Some(BlockClass::AssistantResponse) if !self.assistant_parts.is_empty() => {
                out.push(ResponseItem::AssistantResponse {
                    agent_instance_hierarchy: std::mem::take(&mut self.aih),
                    response_id: std::mem::take(&mut self.rid),
                    parts: std::mem::take(&mut self.assistant_parts),
                });
            }
            Some(BlockClass::RequestMessageAssistant) if !self.assistant_parts.is_empty() => {
                out.push(ResponseItem::RequestMessageAssistant {
                    agent_instance_hierarchy: std::mem::take(&mut self.aih),
                    response_id: std::mem::take(&mut self.rid),
                    parts: std::mem::take(&mut self.assistant_parts),
                });
            }
            Some(BlockClass::ToolResponse) if !self.tool_parts.is_empty() => {
                out.push(ResponseItem::ToolResponse {
                    agent_instance_hierarchy: std::mem::take(&mut self.aih),
                    response_id: std::mem::take(&mut self.rid),
                    tool_call_id: self.tool_call_id.take().unwrap_or_default(),
                    parts: std::mem::take(&mut self.tool_parts),
                });
            }
            Some(BlockClass::RequestMessageTool) if !self.tool_parts.is_empty() => {
                out.push(ResponseItem::RequestMessageTool {
                    agent_instance_hierarchy: std::mem::take(&mut self.aih),
                    response_id: std::mem::take(&mut self.rid),
                    tool_call_id: self.tool_call_id.take().unwrap_or_default(),
                    parts: std::mem::take(&mut self.tool_parts),
                });
            }
            Some(BlockClass::RequestMessageUser) if !self.user_parts.is_empty() => {
                out.push(ResponseItem::RequestMessageUser {
                    agent_instance_hierarchy: std::mem::take(&mut self.aih),
                    response_id: std::mem::take(&mut self.rid),
                    parts: std::mem::take(&mut self.user_parts),
                });
            }
            Some(BlockClass::VectorRequestChoices) => {
                self.finish_choice();
                if !self.choices.is_empty() {
                    out.push(ResponseItem::VectorRequestChoices {
                        agent_instance_hierarchy: std::mem::take(&mut self.aih),
                        response_id: std::mem::take(&mut self.rid),
                        choices: std::mem::take(&mut self.choices),
                    });
                }
            }
            _ => {}
        }
        *self = BlockAccum::default();
    }
}

/// Walk `rows` (already sorted by `id` ASC) and coalesce into
/// `ResponseItem`s. Pure / deterministic. Request-blob marker rows are
/// skipped (still written for `read id`, hidden from `list`). Every
/// block's `response_id` is the AGENT-COMPLETION's id — the writer
/// keyed all request_message / choice / vote rows by the per-agent
/// `agent_completion_chunk` id, never the vector/task id.
fn coalesce_into_blocks(rows: Vec<MsgRow>) -> Vec<ResponseItem> {
    let mut out: Vec<ResponseItem> = Vec::new();
    let mut acc = BlockAccum::default();

    for row in rows {
        let class = block_class(row.table_kind);

        match class {
            // Request-blob marker: hidden from list. Flush + skip.
            BlockClass::SkippedRequestBlob => {
                acc.flush(&mut out);
                continue;
            }
            // Single inline row: this agent's vote (closer).
            BlockClass::VectorResponseVote => {
                acc.flush(&mut out);
                let vote: Vec<rust_decimal::Decimal> = row
                    .vote
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                out.push(ResponseItem::VectorResponseVote {
                    agent_instance_hierarchy: row.agent_instance_hierarchy,
                    response_id: row.response_id,
                    vote,
                });
                continue;
            }
            _ => {}
        }

        // Multi-row block. Boundary tuple: (class, aih, rid) — plus
        // sender+mq_id for ClientNotification, tool_call_id for the two
        // tool classes.
        let boundary = acc.class != Some(class)
            || acc.aih != row.agent_instance_hierarchy
            || acc.rid != row.response_id
            || (class == BlockClass::ClientNotification
                && (acc.sender.as_deref() != row.sender_agent_instance_hierarchy.as_deref()
                    || acc.mq_id != row.message_queue_id))
            || ((class == BlockClass::ToolResponse
                || class == BlockClass::RequestMessageTool)
                && acc.tool_call_id.as_deref() != row.tool_call_id.as_deref());
        if boundary {
            acc.flush(&mut out);
            acc.class = Some(class);
            acc.aih = row.agent_instance_hierarchy.clone();
            acc.rid = row.response_id.clone();
            match class {
                BlockClass::ClientNotification => {
                    acc.sender = row.sender_agent_instance_hierarchy.clone();
                    acc.mq_id = row.message_queue_id;
                    acc.timestamp_queued = row.timestamp_queued;
                    acc.key = row.message_queue_key.clone();
                }
                BlockClass::ToolResponse | BlockClass::RequestMessageTool => {
                    acc.tool_call_id = row.tool_call_id.clone();
                }
                _ => {}
            }
        }

        match class {
            BlockClass::ClientNotification => {
                let r#type = client_notification_kind(row.table_kind)
                    .expect("class invariant: ClientNotification maps to message_queue_*");
                acc.notification_parts.push(ClientNotificationPart {
                    id: row.id,
                    delivered_at: unix_to_rfc3339(row.timestamp_delivered),
                    r#type,
                });
            }
            BlockClass::AssistantResponse | BlockClass::RequestMessageAssistant => {
                let part = assistant_response_part(&row)
                    .expect("class invariant: (request) assistant maps to assistant tables");
                acc.assistant_parts.push(part);
            }
            BlockClass::ToolResponse | BlockClass::RequestMessageTool => {
                let r#type = tool_response_kind(row.table_kind)
                    .expect("class invariant: (request) tool maps to tool content tables");
                acc.tool_parts.push(ToolResponsePart {
                    id: row.id,
                    delivered_at: unix_to_rfc3339(row.timestamp_delivered),
                    r#type,
                });
            }
            BlockClass::RequestMessageUser => {
                let r#type = request_message_user_kind(row.table_kind)
                    .expect("class invariant: RequestMessageUser maps to user content tables");
                acc.user_parts.push(RequestMessageUserPart {
                    id: row.id,
                    delivered_at: unix_to_rfc3339(row.timestamp_delivered),
                    r#type,
                });
            }
            BlockClass::VectorRequestChoices => {
                let r#type = vector_choice_kind(row.table_kind)
                    .expect("class invariant: VectorRequestChoices maps to choice content tables");
                if acc.cur_choice_index != row.row_index {
                    acc.finish_choice();
                    acc.cur_choice_index = row.row_index;
                    acc.cur_choice_key = row.choice_key.clone();
                }
                acc.cur_choice_parts.push(VectorRequestChoicePart {
                    id: row.id,
                    delivered_at: unix_to_rfc3339(row.timestamp_delivered),
                    r#type,
                });
            }
            BlockClass::SkippedRequestBlob | BlockClass::VectorResponseVote => {
                unreachable!("handled by the early `match class` above")
            }
        }
    }

    acc.flush(&mut out);
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
                s.tool_call_id, s.row_sub_index, \
                s.row_index, s.choice_key, s.vote \
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
