//! Row-shape types: [`RowTable`] / [`MessageTable`] / [`RowValue`] /
//! [`RowKey`] / [`OwnedRowKey`] / [`RowBody`].
//!
//! - [`RowValue<'a>`] — what the chunk-row iterators yield. Borrowed
//!   sum type, one variant per streaming-content table. Every variant
//!   carries `response_id` (the enclosing agent_completion_chunk's id)
//!   AND `agent_instance_hierarchy` (the enclosing chunk's spawned
//!   agent id) so the writer can address `logs.messages` /
//!   `logs.messages_queue` without going back to the chunk.
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

/// Every table in the `logs.*` schema. Matches `schema.sql` 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowTable {
    AgentCompletionRequests,
    AgentCompletionResponses,
    VectorCompletionRequests,
    VectorCompletionResponses,
    FunctionExecutionRequests,
    FunctionExecutionResponses,

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

/// The subset of [`RowTable`] that produces a `logs.messages` event
/// row when written. Maps 1:1 to the postgres `logs.message_table`
/// ENUM in `schema.sql` — same names, same order. The three
/// response-blob tables are intentionally absent; they're not events,
/// just the latest snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "logs.message_table", rename_all = "snake_case")]
pub enum MessageTable {
    AgentCompletionRequest,
    VectorCompletionRequest,
    FunctionExecutionRequest,
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

impl RowTable {
    /// The [`MessageTable`] for this table's events. Returns `None` for
    /// the three response-blob tables (which don't emit messages).
    pub fn message_table(self) -> Option<MessageTable> {
        Some(match self {
            RowTable::AgentCompletionRequests => MessageTable::AgentCompletionRequest,
            RowTable::VectorCompletionRequests => MessageTable::VectorCompletionRequest,
            RowTable::FunctionExecutionRequests => MessageTable::FunctionExecutionRequest,
            RowTable::ToolResponse => MessageTable::ToolResponse,
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
            RowTable::AgentCompletionRequests => "logs.agent_completion_requests",
            RowTable::AgentCompletionResponses => "logs.agent_completion_responses",
            RowTable::VectorCompletionRequests => "logs.vector_completion_requests",
            RowTable::VectorCompletionResponses => "logs.vector_completion_responses",
            RowTable::FunctionExecutionRequests => "logs.function_execution_requests",
            RowTable::FunctionExecutionResponses => "logs.function_execution_responses",
            RowTable::ToolResponse => "logs.tool_response",
            RowTable::AssistantResponseRefusal => "logs.assistant_response_refusal",
            RowTable::AssistantResponseReasoning => "logs.assistant_response_reasoning",
            RowTable::AssistantResponseToolCalls => "logs.assistant_response_tool_calls",
            RowTable::AssistantResponseContentText => "logs.assistant_response_content_text",
            RowTable::AssistantResponseContentImage => "logs.assistant_response_content_image",
            RowTable::AssistantResponseContentAudio => "logs.assistant_response_content_audio",
            RowTable::AssistantResponseContentVideo => "logs.assistant_response_content_video",
            RowTable::AssistantResponseContentFile => "logs.assistant_response_content_file",
            RowTable::ToolResponseContentText => "logs.tool_response_content_text",
            RowTable::ToolResponseContentImage => "logs.tool_response_content_image",
            RowTable::ToolResponseContentAudio => "logs.tool_response_content_audio",
            RowTable::ToolResponseContentVideo => "logs.tool_response_content_video",
            RowTable::ToolResponseContentFile => "logs.tool_response_content_file",
        }
    }
}

/// One streaming-content row to INSERT or UPDATE. Borrowed: every
/// variant lifts string / media payloads from the owning chunk by
/// reference. Every variant also carries the enclosing chunk's
/// `agent_instance_hierarchy` so the writer can populate
/// `logs.messages.agent_instance_hierarchy` and key the
/// `logs.messages_queue` downgrade against the right spawned agent.
#[derive(Debug, Clone)]
pub enum RowValue<'a> {
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
        is_input: bool,
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
        is_input: bool,
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
    /// always have one.
    pub fn message_table(&self) -> MessageTable {
        self.table()
            .message_table()
            .expect("RowValue variants only cover streaming-content tables")
    }

    /// `response_id` borrowed from the enclosing agent-completion chunk.
    pub fn response_id(&self) -> &'a str {
        match self {
            RowValue::ToolResponse { response_id, .. }
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
            RowValue::ToolResponse { agent_instance_hierarchy, .. }
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
            RowValue::ToolResponse { .. }
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
                RowValue::AssistantResponseContentVideo { video_url: a, is_input: ai, .. },
                RowBody::AssistantContentVideo { video_url: b, is_input: bi },
            ) => *a == b && ai == bi,
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
                RowValue::ToolResponseContentVideo { video_url: a, is_input: ai, .. },
                RowBody::ToolContentVideo { video_url: b, is_input: bi },
            ) => *a == b && ai == bi,
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
            RowValue::AssistantResponseContentVideo { video_url, is_input, .. } => {
                RowBody::AssistantContentVideo {
                    video_url: (*video_url).clone(),
                    is_input: *is_input,
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
            RowValue::ToolResponseContentVideo { video_url, is_input, .. } => RowBody::ToolContentVideo {
                video_url: (*video_url).clone(),
                is_input: *is_input,
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
    ToolResponse { tool_call_id: String },
    AssistantRefusal { text: String },
    AssistantReasoning { text: String },
    AssistantToolCall { tool_call_id: String, arguments: String },
    AssistantContentText { text: String },
    AssistantContentImage { image_url: ImageUrl },
    AssistantContentAudio { input_audio: InputAudio },
    AssistantContentVideo { video_url: VideoUrl, is_input: bool },
    AssistantContentFile { file: File },
    ToolContentText { text: String },
    ToolContentImage { image_url: ImageUrl },
    ToolContentAudio { input_audio: InputAudio },
    ToolContentVideo { video_url: VideoUrl, is_input: bool },
    ToolContentFile { file: File },
}
