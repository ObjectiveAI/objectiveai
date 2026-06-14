//! Row-shape types: [`RowTable`] / [`MessageTable`] / [`RowValue`] /
//! [`RowKey`] / [`OwnedRowKey`] / [`RowBody`].
//!
//! - [`RowValue<'a>`] — what the chunk-row iterators yield. Borrowed
//!   sum type, one variant per streaming-content table. Every variant
//!   carries `response_id` (the enclosing agent_completion_chunk's id)
//!   AND `agent_instance_hierarchy` (the enclosing chunk's spawned
//!   agent id) so the writer can address `objectiveai.messages` /
//!   `objectiveai.messages_queue` without going back to the chunk.
//! - [`RowKey<'a>`] — what [`RowValue::key`] returns. Borrowed key
//!   used for shadow-map lookups; no allocation.
//! - [`OwnedRowKey`] — same variants as `RowKey`, but with `String`
//!   instead of `&str`. Only built on insert (cold path).
//! - [`RowBody`] — owned body sum, one variant per table. Used by the
//!   shadow to store the last-written body so the next tick can
//!   [`RowValue::body_eq`] against it without re-hashing or
//!   re-serializing.
//!
//! Hash invariant: `RowKey<'a>` and `OwnedRowKey` MUST produce the
//! same `u64` hash for the same logical key. Since `&str` and `String`
//! hash identically and the variant tag is determined by source
//! position (matching variants are at matching positions in the two
//! enums), `derive(Hash)` is sufficient — no manual impl needed.

use objectiveai_sdk::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};

/// Every table in the `logs.*` schema, plus the synthetic
/// `MessageQueueContent` variant for queue-consumption rows.
/// The latter writes to `objectiveai.messages` with a `"table"` value
/// chosen at write time via SQL CASE (one of `message_queue_text`,
/// `_image`, `_audio`, `_video`, `_file`) — the Rust-side variant
/// is kind-less because the dispatch happens in SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowTable {
    AgentCompletionRequests,
    AgentCompletionResponses,
    VectorCompletionRequests,
    VectorCompletionResponses,
    FunctionExecutionRequests,
    FunctionExecutionResponses,

    /// Synthetic — kind-less at the Rust level. The writer's
    /// helper picks the per-kind `objectiveai.message_table` enum value
    /// via SQL CASE against `message_queue_contents.kind` and
    /// flips the parent `message_queue.active = FALSE` in the
    /// same statement.
    MessageQueueContent,

    ToolResponse,

    AssistantResponseRefusal,
    AssistantResponseReasoning,
    AssistantResponseToolCalls,

    AssistantResponseContentText,
    AssistantResponseContentImage,
    AssistantResponseContentAudio,
    AssistantResponseContentVideo,
    AssistantResponseContentFile,

    ToolResponseContentText,
    ToolResponseContentImage,
    ToolResponseContentAudio,
    ToolResponseContentVideo,
    ToolResponseContentFile,
}

/// The subset of [`RowTable`] that produces a `objectiveai.messages` event
/// row when written. Maps 1:1 to the postgres `objectiveai.message_table`
/// ENUM in `schema.sql` — same names, same order. The three
/// response-blob tables are intentionally absent; they're not events,
/// just the latest snapshot. `tool_response` is also absent: its head
/// row is written purely as the `tool_call_id` lookup for tool-response
/// content rows (JOINed at read time) and emits no event of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "objectiveai.message_table", rename_all = "snake_case")]
pub enum MessageTable {
    AgentCompletionRequest,
    VectorCompletionRequest,
    FunctionExecutionRequest,
    MessageQueueText,
    MessageQueueImage,
    MessageQueueAudio,
    MessageQueueVideo,
    MessageQueueFile,
    AssistantResponseRefusal,
    AssistantResponseReasoning,
    AssistantResponseToolCalls,
    AssistantResponseContentText,
    AssistantResponseContentImage,
    AssistantResponseContentAudio,
    AssistantResponseContentVideo,
    AssistantResponseContentFile,
    ToolResponseContentText,
    ToolResponseContentImage,
    ToolResponseContentAudio,
    ToolResponseContentVideo,
    ToolResponseContentFile,
}

impl MessageTable {
    /// snake_case schema name — matches the postgres enum value
    /// declared in `schema.sql`. Used to build `IN (...)` filter
    /// clauses inline since sqlx doesn't auto-derive
    /// `PgHasArrayType` for custom enum types.
    pub fn schema_name(self) -> &'static str {
        match self {
            MessageTable::AgentCompletionRequest => "agent_completion_request",
            MessageTable::VectorCompletionRequest => "vector_completion_request",
            MessageTable::FunctionExecutionRequest => "function_execution_request",
            MessageTable::MessageQueueText => "message_queue_text",
            MessageTable::MessageQueueImage => "message_queue_image",
            MessageTable::MessageQueueAudio => "message_queue_audio",
            MessageTable::MessageQueueVideo => "message_queue_video",
            MessageTable::MessageQueueFile => "message_queue_file",
            MessageTable::AssistantResponseRefusal => "assistant_response_refusal",
            MessageTable::AssistantResponseReasoning => "assistant_response_reasoning",
            MessageTable::AssistantResponseToolCalls => "assistant_response_tool_calls",
            MessageTable::AssistantResponseContentText => "assistant_response_content_text",
            MessageTable::AssistantResponseContentImage => "assistant_response_content_image",
            MessageTable::AssistantResponseContentAudio => "assistant_response_content_audio",
            MessageTable::AssistantResponseContentVideo => "assistant_response_content_video",
            MessageTable::AssistantResponseContentFile => "assistant_response_content_file",
            MessageTable::ToolResponseContentText => "tool_response_content_text",
            MessageTable::ToolResponseContentImage => "tool_response_content_image",
            MessageTable::ToolResponseContentAudio => "tool_response_content_audio",
            MessageTable::ToolResponseContentVideo => "tool_response_content_video",
            MessageTable::ToolResponseContentFile => "tool_response_content_file",
        }
    }
}

impl RowTable {
    /// The [`MessageTable`] for this table's events. Returns `None` for
    /// the three response-blob tables (which don't emit messages).
    pub fn message_table(self) -> Option<MessageTable> {
        Some(match self {
            RowTable::AgentCompletionRequests => MessageTable::AgentCompletionRequest,
            RowTable::VectorCompletionRequests => MessageTable::VectorCompletionRequest,
            RowTable::FunctionExecutionRequests => MessageTable::FunctionExecutionRequest,
            // MessageQueueContent's table value is resolved at
            // write time via SQL CASE — the standard
            // `message_table()` path can't pick from the 5
            // per-kind variants without the kind. Callers writing
            // these rows skip this helper.
            RowTable::MessageQueueContent => return None,
            // The tool-response head row is written to
            // `objectiveai.tool_response` purely as the `tool_call_id`
            // lookup for its content rows (JOINed at read time). It
            // emits no `messages` event, so it's never addressable by
            // `agents logs read id` and never appears as its own part
            // in `read all`. write.rs MUST early-branch this variant
            // before calling `RowValue::message_table()` (which
            // `.expect()`s a Some) — see `insert_value`/`update_value`.
            RowTable::ToolResponse => return None,
            RowTable::AssistantResponseRefusal => MessageTable::AssistantResponseRefusal,
            RowTable::AssistantResponseReasoning => MessageTable::AssistantResponseReasoning,
            RowTable::AssistantResponseToolCalls => MessageTable::AssistantResponseToolCalls,
            RowTable::AssistantResponseContentText => MessageTable::AssistantResponseContentText,
            RowTable::AssistantResponseContentImage => MessageTable::AssistantResponseContentImage,
            RowTable::AssistantResponseContentAudio => MessageTable::AssistantResponseContentAudio,
            RowTable::AssistantResponseContentVideo => MessageTable::AssistantResponseContentVideo,
            RowTable::AssistantResponseContentFile => MessageTable::AssistantResponseContentFile,
            RowTable::ToolResponseContentText => MessageTable::ToolResponseContentText,
            RowTable::ToolResponseContentImage => MessageTable::ToolResponseContentImage,
            RowTable::ToolResponseContentAudio => MessageTable::ToolResponseContentAudio,
            RowTable::ToolResponseContentVideo => MessageTable::ToolResponseContentVideo,
            RowTable::ToolResponseContentFile => MessageTable::ToolResponseContentFile,
            RowTable::AgentCompletionResponses
            | RowTable::VectorCompletionResponses
            | RowTable::FunctionExecutionResponses => return None,
        })
    }

    pub fn fq_name(self) -> &'static str {
        match self {
            RowTable::AgentCompletionRequests => "objectiveai.agent_completion_requests",
            RowTable::AgentCompletionResponses => "objectiveai.agent_completion_responses",
            RowTable::VectorCompletionRequests => "objectiveai.vector_completion_requests",
            RowTable::VectorCompletionResponses => "objectiveai.vector_completion_responses",
            RowTable::FunctionExecutionRequests => "objectiveai.function_execution_requests",
            RowTable::FunctionExecutionResponses => "objectiveai.function_execution_responses",
            // Synthetic — kind chosen at write time via SQL CASE.
            // No per-kind table name surfaces through fq_name();
            // the writer's helper builds its SQL inline.
            RowTable::MessageQueueContent => "message_queue_contents",
            RowTable::ToolResponse => "objectiveai.tool_response",
            RowTable::AssistantResponseRefusal => "objectiveai.assistant_response_refusal",
            RowTable::AssistantResponseReasoning => "objectiveai.assistant_response_reasoning",
            RowTable::AssistantResponseToolCalls => "objectiveai.assistant_response_tool_calls",
            RowTable::AssistantResponseContentText => "objectiveai.assistant_response_content_text",
            RowTable::AssistantResponseContentImage => "objectiveai.assistant_response_content_image",
            RowTable::AssistantResponseContentAudio => "objectiveai.assistant_response_content_audio",
            RowTable::AssistantResponseContentVideo => "objectiveai.assistant_response_content_video",
            RowTable::AssistantResponseContentFile => "objectiveai.assistant_response_content_file",
            RowTable::ToolResponseContentText => "objectiveai.tool_response_content_text",
            RowTable::ToolResponseContentImage => "objectiveai.tool_response_content_image",
            RowTable::ToolResponseContentAudio => "objectiveai.tool_response_content_audio",
            RowTable::ToolResponseContentVideo => "objectiveai.tool_response_content_video",
            RowTable::ToolResponseContentFile => "objectiveai.tool_response_content_file",
        }
    }
}

/// One streaming-content row to INSERT or UPDATE. Borrowed: every
/// variant lifts string / media payloads from the owning chunk by
/// reference. Every variant also carries the enclosing chunk's
/// `agent_instance_hierarchy` so the writer can populate
/// `objectiveai.messages.agent_instance_hierarchy` and key the
/// `objectiveai.messages_queue` downgrade against the right spawned agent.
#[derive(Debug, Clone)]
pub enum RowValue<'a> {
    /// Consumption signal: the API stamped this
    /// `message_queue_contents.id` onto the chunk's
    /// `request_message_ids` field. The writer's helper:
    /// 1) resolves the row's kind via SQL CASE against
    ///    `message_queue_contents.kind`,
    /// 2) `UPDATE objectiveai.message_queue SET active=FALSE WHERE id =
    ///    (SELECT message_queue_id FROM objectiveai.message_queue_contents
    ///     WHERE id = $content_id) AND active=TRUE`, and
    /// 3) `INSERT objectiveai.messages` with `"table"` picked from the
    ///    five `message_queue_*` enum values matching the kind
    ///    and `row_index = content_id`, so the read path
    ///    dispatches directly to the right per-kind table.
    ///
    /// Multiple content_ids sharing one parent fire the flip
    /// once; the rest are no-ops via the `AND active=TRUE` guard.
    ///
    /// Iterators yield these AHEAD of the per-message content
    /// rows so the log chronicles consumption before the body
    /// the agent produced.
    MessageQueueContent {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        message_queue_content_id: i64,
    },
    ToolResponse {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        tool_call_id: &'a str,
    },
    AssistantResponseRefusal {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        text: &'a str,
    },
    AssistantResponseReasoning {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        text: &'a str,
    },
    AssistantResponseToolCalls {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        tool_call_index: u64,
        tool_call_id: &'a str,
        function_name: &'a str,
        arguments: &'a str,
    },

    AssistantResponseContentText {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        text: &'a str,
    },
    AssistantResponseContentImage {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        image_url: &'a ImageUrl,
    },
    AssistantResponseContentAudio {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        input_audio: &'a InputAudio,
    },
    AssistantResponseContentVideo {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        video_url: &'a VideoUrl,
    },
    AssistantResponseContentFile {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        file: &'a File,
    },

    ToolResponseContentText {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        text: &'a str,
    },
    ToolResponseContentImage {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        image_url: &'a ImageUrl,
    },
    ToolResponseContentAudio {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        input_audio: &'a InputAudio,
    },
    ToolResponseContentVideo {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        video_url: &'a VideoUrl,
    },
    ToolResponseContentFile {
        response_id: &'a str,
        agent_instance_hierarchy: &'a str,
        index: u64,
        part_index: u64,
        file: &'a File,
    },
}

impl<'a> RowValue<'a> {
    pub fn table(&self) -> RowTable {
        match self {
            RowValue::MessageQueueContent { .. } => RowTable::MessageQueueContent,
            RowValue::ToolResponse { .. } => RowTable::ToolResponse,
            RowValue::AssistantResponseRefusal { .. } => RowTable::AssistantResponseRefusal,
            RowValue::AssistantResponseReasoning { .. } => RowTable::AssistantResponseReasoning,
            RowValue::AssistantResponseToolCalls { .. } => RowTable::AssistantResponseToolCalls,
            RowValue::AssistantResponseContentText { .. } => RowTable::AssistantResponseContentText,
            RowValue::AssistantResponseContentImage { .. } => RowTable::AssistantResponseContentImage,
            RowValue::AssistantResponseContentAudio { .. } => RowTable::AssistantResponseContentAudio,
            RowValue::AssistantResponseContentVideo { .. } => RowTable::AssistantResponseContentVideo,
            RowValue::AssistantResponseContentFile { .. } => RowTable::AssistantResponseContentFile,
            RowValue::ToolResponseContentText { .. } => RowTable::ToolResponseContentText,
            RowValue::ToolResponseContentImage { .. } => RowTable::ToolResponseContentImage,
            RowValue::ToolResponseContentAudio { .. } => RowTable::ToolResponseContentAudio,
            RowValue::ToolResponseContentVideo { .. } => RowTable::ToolResponseContentVideo,
            RowValue::ToolResponseContentFile { .. } => RowTable::ToolResponseContentFile,
        }
    }

    /// [`MessageTable`] for this row's table. Streaming-content rows
    /// always have one — EXCEPT `RowValue::ToolResponse`, whose head
    /// row emits no `messages` event (it's a `tool_call_id` lookup
    /// only). Callers MUST early-branch `ToolResponse` before calling
    /// this (`write.rs::insert_value`/`update_value` do), or the
    /// `.expect()` below panics.
    pub fn message_table(&self) -> MessageTable {
        self.table()
            .message_table()
            .expect("RowValue variants (except ToolResponse) cover messages-emitting tables")
    }

    /// `response_id` borrowed from the immediately-enclosing
    /// agent-completion chunk's `id`. Even for rows emitted under
    /// `vector_completion_chunk_rows` /
    /// `function_execution_chunk_rows`, the recursion bottoms out
    /// at an agent-completion chunk and uses that chunk's id —
    /// never the outer vector/function wrapper's id. This is what
    /// `agents logs read all` / `read pending` use as the
    /// per-block `response_id` boundary key.
    pub fn response_id(&self) -> &'a str {
        match self {
            RowValue::MessageQueueContent { response_id, .. }
            | RowValue::ToolResponse { response_id, .. }
            | RowValue::AssistantResponseRefusal { response_id, .. }
            | RowValue::AssistantResponseReasoning { response_id, .. }
            | RowValue::AssistantResponseToolCalls { response_id, .. }
            | RowValue::AssistantResponseContentText { response_id, .. }
            | RowValue::AssistantResponseContentImage { response_id, .. }
            | RowValue::AssistantResponseContentAudio { response_id, .. }
            | RowValue::AssistantResponseContentVideo { response_id, .. }
            | RowValue::AssistantResponseContentFile { response_id, .. }
            | RowValue::ToolResponseContentText { response_id, .. }
            | RowValue::ToolResponseContentImage { response_id, .. }
            | RowValue::ToolResponseContentAudio { response_id, .. }
            | RowValue::ToolResponseContentVideo { response_id, .. }
            | RowValue::ToolResponseContentFile { response_id, .. } => response_id,
        }
    }

    /// `agent_instance_hierarchy` borrowed from the enclosing
    /// agent-completion chunk. Every streaming-content row lives
    /// inside an agent completion, so this is always non-NULL.
    pub fn agent_instance_hierarchy(&self) -> &'a str {
        match self {
            RowValue::MessageQueueContent { agent_instance_hierarchy, .. }
            | RowValue::ToolResponse { agent_instance_hierarchy, .. }
            | RowValue::AssistantResponseRefusal { agent_instance_hierarchy, .. }
            | RowValue::AssistantResponseReasoning { agent_instance_hierarchy, .. }
            | RowValue::AssistantResponseToolCalls { agent_instance_hierarchy, .. }
            | RowValue::AssistantResponseContentText { agent_instance_hierarchy, .. }
            | RowValue::AssistantResponseContentImage { agent_instance_hierarchy, .. }
            | RowValue::AssistantResponseContentAudio { agent_instance_hierarchy, .. }
            | RowValue::AssistantResponseContentVideo { agent_instance_hierarchy, .. }
            | RowValue::AssistantResponseContentFile { agent_instance_hierarchy, .. }
            | RowValue::ToolResponseContentText { agent_instance_hierarchy, .. }
            | RowValue::ToolResponseContentImage { agent_instance_hierarchy, .. }
            | RowValue::ToolResponseContentAudio { agent_instance_hierarchy, .. }
            | RowValue::ToolResponseContentVideo { agent_instance_hierarchy, .. }
            | RowValue::ToolResponseContentFile { agent_instance_hierarchy, .. } => {
                agent_instance_hierarchy
            }
        }
    }

    /// `row_index` column value for the postgres `messages` /
    /// `messages_queue` entry. Always populated for streaming-content
    /// rows.
    pub fn row_index(&self) -> i64 {
        match self {
            RowValue::MessageQueueContent { message_queue_content_id, .. } => {
                *message_queue_content_id
            }
            RowValue::ToolResponse { index, .. }
            | RowValue::AssistantResponseRefusal { index, .. }
            | RowValue::AssistantResponseReasoning { index, .. }
            | RowValue::AssistantResponseToolCalls { index, .. }
            | RowValue::AssistantResponseContentText { index, .. }
            | RowValue::AssistantResponseContentImage { index, .. }
            | RowValue::AssistantResponseContentAudio { index, .. }
            | RowValue::AssistantResponseContentVideo { index, .. }
            | RowValue::AssistantResponseContentFile { index, .. }
            | RowValue::ToolResponseContentText { index, .. }
            | RowValue::ToolResponseContentImage { index, .. }
            | RowValue::ToolResponseContentAudio { index, .. }
            | RowValue::ToolResponseContentVideo { index, .. }
            | RowValue::ToolResponseContentFile { index, .. } => *index as i64,
        }
    }

    /// `row_sub_index` column value: `tool_call_index` for tool-call
    /// rows, `part_index` for content-part rows, `None` for
    /// tool_response / assistant refusal / assistant reasoning (whose
    /// shape has no sub-index). The matching SQL column is nullable.
    pub fn row_sub_index(&self) -> Option<i64> {
        match self {
            RowValue::MessageQueueContent { .. }
            | RowValue::ToolResponse { .. }
            | RowValue::AssistantResponseRefusal { .. }
            | RowValue::AssistantResponseReasoning { .. } => None,
            RowValue::AssistantResponseToolCalls { tool_call_index, .. } => {
                Some(*tool_call_index as i64)
            }
            RowValue::AssistantResponseContentText { part_index, .. }
            | RowValue::AssistantResponseContentImage { part_index, .. }
            | RowValue::AssistantResponseContentAudio { part_index, .. }
            | RowValue::AssistantResponseContentVideo { part_index, .. }
            | RowValue::AssistantResponseContentFile { part_index, .. }
            | RowValue::ToolResponseContentText { part_index, .. }
            | RowValue::ToolResponseContentImage { part_index, .. }
            | RowValue::ToolResponseContentAudio { part_index, .. }
            | RowValue::ToolResponseContentVideo { part_index, .. }
            | RowValue::ToolResponseContentFile { part_index, .. } => Some(*part_index as i64),
        }
    }

    /// The borrowed key that identifies this row's slot in postgres.
    /// Hashable + Eq — used by the shadow map to find the previously
    /// stored body without any allocation. Two `RowValue`s targeting
    /// the same row produce equal `RowKey`s.
    pub fn key(&self) -> RowKey<'a> {
        match self {
            RowValue::MessageQueueContent {
                response_id,
                message_queue_content_id,
                ..
            } => RowKey::MessageQueueContent {
                response_id,
                message_queue_content_id: *message_queue_content_id,
            },
            RowValue::ToolResponse { response_id, index, .. } => {
                RowKey::ToolResponse { response_id, index: *index }
            }
            RowValue::AssistantResponseRefusal { response_id, index, .. } => {
                RowKey::AssistantRefusal { response_id, index: *index }
            }
            RowValue::AssistantResponseReasoning { response_id, index, .. } => {
                RowKey::AssistantReasoning { response_id, index: *index }
            }
            RowValue::AssistantResponseToolCalls {
                response_id, index, tool_call_index, ..
            } => RowKey::AssistantToolCall {
                response_id,
                index: *index,
                tool_call_index: *tool_call_index,
            },
            RowValue::AssistantResponseContentText { response_id, index, part_index, .. } => {
                RowKey::AssistantContentText { response_id, index: *index, part_index: *part_index }
            }
            RowValue::AssistantResponseContentImage { response_id, index, part_index, .. } => {
                RowKey::AssistantContentImage { response_id, index: *index, part_index: *part_index }
            }
            RowValue::AssistantResponseContentAudio { response_id, index, part_index, .. } => {
                RowKey::AssistantContentAudio { response_id, index: *index, part_index: *part_index }
            }
            RowValue::AssistantResponseContentVideo { response_id, index, part_index, .. } => {
                RowKey::AssistantContentVideo { response_id, index: *index, part_index: *part_index }
            }
            RowValue::AssistantResponseContentFile { response_id, index, part_index, .. } => {
                RowKey::AssistantContentFile { response_id, index: *index, part_index: *part_index }
            }
            RowValue::ToolResponseContentText { response_id, index, part_index, .. } => {
                RowKey::ToolContentText { response_id, index: *index, part_index: *part_index }
            }
            RowValue::ToolResponseContentImage { response_id, index, part_index, .. } => {
                RowKey::ToolContentImage { response_id, index: *index, part_index: *part_index }
            }
            RowValue::ToolResponseContentAudio { response_id, index, part_index, .. } => {
                RowKey::ToolContentAudio { response_id, index: *index, part_index: *part_index }
            }
            RowValue::ToolResponseContentVideo { response_id, index, part_index, .. } => {
                RowKey::ToolContentVideo { response_id, index: *index, part_index: *part_index }
            }
            RowValue::ToolResponseContentFile { response_id, index, part_index, .. } => {
                RowKey::ToolContentFile { response_id, index: *index, part_index: *part_index }
            }
        }
    }

    /// Field-by-field equality against a stored body. Returns true
    /// when this row would write a byte-identical body — the writer
    /// uses that signal to short-circuit the SQL.
    pub fn body_eq(&self, stored: &RowBody) -> bool {
        match (self, stored) {
            // MessageQueueContent has no body — the row's identity is
            // entirely in the (response_id, content_id) key; shadow
            // skip-dedup makes the second write a no-op via this true.
            (
                RowValue::MessageQueueContent { .. },
                RowBody::MessageQueueContent {},
            ) => true,
            (
                RowValue::ToolResponse { tool_call_id: a, .. },
                RowBody::ToolResponse { tool_call_id: b },
            ) => *a == b.as_str(),
            (
                RowValue::AssistantResponseRefusal { text: a, .. },
                RowBody::AssistantRefusal { text: b },
            ) => *a == b.as_str(),
            (
                RowValue::AssistantResponseReasoning { text: a, .. },
                RowBody::AssistantReasoning { text: b },
            ) => *a == b.as_str(),
            (
                RowValue::AssistantResponseToolCalls { tool_call_id: a, arguments: aa, .. },
                RowBody::AssistantToolCall { tool_call_id: b, arguments: bb },
            ) => *a == b.as_str() && *aa == bb.as_str(),
            (
                RowValue::AssistantResponseContentText { text: a, .. },
                RowBody::AssistantContentText { text: b },
            ) => *a == b.as_str(),
            (
                RowValue::AssistantResponseContentImage { image_url: a, .. },
                RowBody::AssistantContentImage { image_url: b },
            ) => *a == b,
            (
                RowValue::AssistantResponseContentAudio { input_audio: a, .. },
                RowBody::AssistantContentAudio { input_audio: b },
            ) => *a == b,
            (
                RowValue::AssistantResponseContentVideo { video_url: a, .. },
                RowBody::AssistantContentVideo { video_url: b },
            ) => *a == b,
            (
                RowValue::AssistantResponseContentFile { file: a, .. },
                RowBody::AssistantContentFile { file: b },
            ) => *a == b,
            (
                RowValue::ToolResponseContentText { text: a, .. },
                RowBody::ToolContentText { text: b },
            ) => *a == b.as_str(),
            (
                RowValue::ToolResponseContentImage { image_url: a, .. },
                RowBody::ToolContentImage { image_url: b },
            ) => *a == b,
            (
                RowValue::ToolResponseContentAudio { input_audio: a, .. },
                RowBody::ToolContentAudio { input_audio: b },
            ) => *a == b,
            (
                RowValue::ToolResponseContentVideo { video_url: a, .. },
                RowBody::ToolContentVideo { video_url: b },
            ) => *a == b,
            (
                RowValue::ToolResponseContentFile { file: a, .. },
                RowBody::ToolContentFile { file: b },
            ) => *a == b,
            _ => false,
        }
    }

    /// Build an owned [`RowBody`] for storing on Insert / Update. Only
    /// called on the cold path (when the shadow needs to remember a
    /// new value); the Skip path never allocates here.
    pub fn to_body(&self) -> RowBody {
        match self {
            RowValue::MessageQueueContent { .. } => RowBody::MessageQueueContent {},
            RowValue::ToolResponse { tool_call_id, .. } => RowBody::ToolResponse {
                tool_call_id: (*tool_call_id).to_owned(),
            },
            RowValue::AssistantResponseRefusal { text, .. } => RowBody::AssistantRefusal {
                text: (*text).to_owned(),
            },
            RowValue::AssistantResponseReasoning { text, .. } => RowBody::AssistantReasoning {
                text: (*text).to_owned(),
            },
            RowValue::AssistantResponseToolCalls { tool_call_id, arguments, .. } => {
                RowBody::AssistantToolCall {
                    tool_call_id: (*tool_call_id).to_owned(),
                    arguments: (*arguments).to_owned(),
                }
            }
            RowValue::AssistantResponseContentText { text, .. } => RowBody::AssistantContentText {
                text: (*text).to_owned(),
            },
            RowValue::AssistantResponseContentImage { image_url, .. } => {
                RowBody::AssistantContentImage { image_url: (*image_url).clone() }
            }
            RowValue::AssistantResponseContentAudio { input_audio, .. } => {
                RowBody::AssistantContentAudio { input_audio: (*input_audio).clone() }
            }
            RowValue::AssistantResponseContentVideo { video_url, .. } => {
                RowBody::AssistantContentVideo {
                    video_url: (*video_url).clone(),
                }
            }
            RowValue::AssistantResponseContentFile { file, .. } => {
                RowBody::AssistantContentFile { file: (*file).clone() }
            }
            RowValue::ToolResponseContentText { text, .. } => RowBody::ToolContentText {
                text: (*text).to_owned(),
            },
            RowValue::ToolResponseContentImage { image_url, .. } => {
                RowBody::ToolContentImage { image_url: (*image_url).clone() }
            }
            RowValue::ToolResponseContentAudio { input_audio, .. } => {
                RowBody::ToolContentAudio { input_audio: (*input_audio).clone() }
            }
            RowValue::ToolResponseContentVideo { video_url, .. } => RowBody::ToolContentVideo {
                video_url: (*video_url).clone(),
            },
            RowValue::ToolResponseContentFile { file, .. } => RowBody::ToolContentFile {
                file: (*file).clone(),
            },
        }
    }
}

/// Boxed-iterator alias used at the recursive boundaries
/// (function execution → vector completion → agent completion).
/// One Box per recursive descent, never per leaf row.
pub type RowsIter<'a> = Box<dyn Iterator<Item = RowValue<'a>> + Send + 'a>;

// ---------------------------------------------------------------------
// Shadow keys (borrowed + owned)
// ---------------------------------------------------------------------

/// Borrowed key that identifies one row's slot in postgres. Two
/// `RowKey`s with equal variant + fields hash identically to an
/// [`OwnedRowKey`] with the matching shape — that invariant lets the
/// shadow's hashbrown `raw_entry_mut` look up by borrowed key without
/// converting to owned first.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum RowKey<'a> {
    /// `message_queue_contents.id` is globally unique across
    /// kinds, so the key needs no kind discriminator — only the
    /// id (the kind is recovered from `RowValue` for write
    /// dispatch).
    MessageQueueContent { response_id: &'a str, message_queue_content_id: i64 },
    ToolResponse { response_id: &'a str, index: u64 },
    AssistantRefusal { response_id: &'a str, index: u64 },
    AssistantReasoning { response_id: &'a str, index: u64 },
    AssistantToolCall { response_id: &'a str, index: u64, tool_call_index: u64 },
    AssistantContentText { response_id: &'a str, index: u64, part_index: u64 },
    AssistantContentImage { response_id: &'a str, index: u64, part_index: u64 },
    AssistantContentAudio { response_id: &'a str, index: u64, part_index: u64 },
    AssistantContentVideo { response_id: &'a str, index: u64, part_index: u64 },
    AssistantContentFile { response_id: &'a str, index: u64, part_index: u64 },
    ToolContentText { response_id: &'a str, index: u64, part_index: u64 },
    ToolContentImage { response_id: &'a str, index: u64, part_index: u64 },
    ToolContentAudio { response_id: &'a str, index: u64, part_index: u64 },
    ToolContentVideo { response_id: &'a str, index: u64, part_index: u64 },
    ToolContentFile { response_id: &'a str, index: u64, part_index: u64 },
}

/// Owned counterpart to [`RowKey`]. Stored in the shadow map. Built
/// only on Insert.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum OwnedRowKey {
    MessageQueueContent { response_id: String, message_queue_content_id: i64 },
    ToolResponse { response_id: String, index: u64 },
    AssistantRefusal { response_id: String, index: u64 },
    AssistantReasoning { response_id: String, index: u64 },
    AssistantToolCall { response_id: String, index: u64, tool_call_index: u64 },
    AssistantContentText { response_id: String, index: u64, part_index: u64 },
    AssistantContentImage { response_id: String, index: u64, part_index: u64 },
    AssistantContentAudio { response_id: String, index: u64, part_index: u64 },
    AssistantContentVideo { response_id: String, index: u64, part_index: u64 },
    AssistantContentFile { response_id: String, index: u64, part_index: u64 },
    ToolContentText { response_id: String, index: u64, part_index: u64 },
    ToolContentImage { response_id: String, index: u64, part_index: u64 },
    ToolContentAudio { response_id: String, index: u64, part_index: u64 },
    ToolContentVideo { response_id: String, index: u64, part_index: u64 },
    ToolContentFile { response_id: String, index: u64, part_index: u64 },
}

impl<'a> RowKey<'a> {
    pub fn matches_owned(&self, owned: &OwnedRowKey) -> bool {
        match (self, owned) {
            (
                RowKey::MessageQueueContent { response_id: a, message_queue_content_id: ai },
                OwnedRowKey::MessageQueueContent { response_id: b, message_queue_content_id: bi },
            ) => *a == b.as_str() && ai == bi,
            (
                RowKey::ToolResponse { response_id: a, index: ai },
                OwnedRowKey::ToolResponse { response_id: b, index: bi },
            ) => *a == b.as_str() && ai == bi,
            (
                RowKey::AssistantRefusal { response_id: a, index: ai },
                OwnedRowKey::AssistantRefusal { response_id: b, index: bi },
            ) => *a == b.as_str() && ai == bi,
            (
                RowKey::AssistantReasoning { response_id: a, index: ai },
                OwnedRowKey::AssistantReasoning { response_id: b, index: bi },
            ) => *a == b.as_str() && ai == bi,
            (
                RowKey::AssistantToolCall { response_id: a, index: ai, tool_call_index: at },
                OwnedRowKey::AssistantToolCall { response_id: b, index: bi, tool_call_index: bt },
            ) => *a == b.as_str() && ai == bi && at == bt,
            (
                RowKey::AssistantContentText { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::AssistantContentText { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            (
                RowKey::AssistantContentImage { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::AssistantContentImage { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            (
                RowKey::AssistantContentAudio { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::AssistantContentAudio { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            (
                RowKey::AssistantContentVideo { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::AssistantContentVideo { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            (
                RowKey::AssistantContentFile { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::AssistantContentFile { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            (
                RowKey::ToolContentText { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::ToolContentText { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            (
                RowKey::ToolContentImage { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::ToolContentImage { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            (
                RowKey::ToolContentAudio { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::ToolContentAudio { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            (
                RowKey::ToolContentVideo { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::ToolContentVideo { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            (
                RowKey::ToolContentFile { response_id: a, index: ai, part_index: ap },
                OwnedRowKey::ToolContentFile { response_id: b, index: bi, part_index: bp },
            ) => *a == b.as_str() && ai == bi && ap == bp,
            _ => false,
        }
    }

    pub fn to_owned_key(&self) -> OwnedRowKey {
        match self {
            RowKey::MessageQueueContent { response_id, message_queue_content_id } => {
                OwnedRowKey::MessageQueueContent {
                    response_id: (*response_id).to_owned(),
                    message_queue_content_id: *message_queue_content_id,
                }
            }
            RowKey::ToolResponse { response_id, index } => OwnedRowKey::ToolResponse {
                response_id: (*response_id).to_owned(),
                index: *index,
            },
            RowKey::AssistantRefusal { response_id, index } => OwnedRowKey::AssistantRefusal {
                response_id: (*response_id).to_owned(),
                index: *index,
            },
            RowKey::AssistantReasoning { response_id, index } => OwnedRowKey::AssistantReasoning {
                response_id: (*response_id).to_owned(),
                index: *index,
            },
            RowKey::AssistantToolCall { response_id, index, tool_call_index } => {
                OwnedRowKey::AssistantToolCall {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    tool_call_index: *tool_call_index,
                }
            }
            RowKey::AssistantContentText { response_id, index, part_index } => {
                OwnedRowKey::AssistantContentText {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
            RowKey::AssistantContentImage { response_id, index, part_index } => {
                OwnedRowKey::AssistantContentImage {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
            RowKey::AssistantContentAudio { response_id, index, part_index } => {
                OwnedRowKey::AssistantContentAudio {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
            RowKey::AssistantContentVideo { response_id, index, part_index } => {
                OwnedRowKey::AssistantContentVideo {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
            RowKey::AssistantContentFile { response_id, index, part_index } => {
                OwnedRowKey::AssistantContentFile {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
            RowKey::ToolContentText { response_id, index, part_index } => {
                OwnedRowKey::ToolContentText {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
            RowKey::ToolContentImage { response_id, index, part_index } => {
                OwnedRowKey::ToolContentImage {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
            RowKey::ToolContentAudio { response_id, index, part_index } => {
                OwnedRowKey::ToolContentAudio {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
            RowKey::ToolContentVideo { response_id, index, part_index } => {
                OwnedRowKey::ToolContentVideo {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
            RowKey::ToolContentFile { response_id, index, part_index } => {
                OwnedRowKey::ToolContentFile {
                    response_id: (*response_id).to_owned(),
                    index: *index,
                    part_index: *part_index,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------
// Shadow body
// ---------------------------------------------------------------------

/// Owned body stored in the shadow map. Compared by `PartialEq`
/// against an incoming [`RowValue`] via [`RowValue::body_eq`].
#[derive(Debug, Clone, PartialEq)]
pub enum RowBody {
    /// Empty marker — `MessageQueueContent` rows have no body; the
    /// shadow uses presence-only for skip detection (body_eq
    /// returns true for any matching key, so the second sight of
    /// the same content_id is treated as Skip).
    MessageQueueContent {},
    ToolResponse { tool_call_id: String },
    AssistantRefusal { text: String },
    AssistantReasoning { text: String },
    AssistantToolCall { tool_call_id: String, arguments: String },
    AssistantContentText { text: String },
    AssistantContentImage { image_url: ImageUrl },
    AssistantContentAudio { input_audio: InputAudio },
    AssistantContentVideo { video_url: VideoUrl },
    AssistantContentFile { file: File },
    ToolContentText { text: String },
    ToolContentImage { image_url: ImageUrl },
    ToolContentAudio { input_audio: InputAudio },
    ToolContentVideo { video_url: VideoUrl },
    ToolContentFile { file: File },
}
