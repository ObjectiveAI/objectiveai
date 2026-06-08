//! SDK-side `logs` types.
//!
//! Holds the two enums every postgres-log writer dispatches on:
//!
//! - [`LogTable`] — closed list of every table in the `logs.*` schema.
//! - [`LogValue`] — borrowed (`'a`) sum type of every row shape the
//!   writer can be asked to UPSERT. One variant per table.
//!
//! Each chunk type (`AgentCompletionChunk`, `VectorCompletionChunk`,
//! `FunctionExecutionChunk`) exposes a `log_rows(&self)` method that
//! returns an iterator over `(LogTable, LogValue<'_>)` pairs. The
//! iterator never collects: it yields one row at a time as the writer
//! pulls. Recursive structures (function executions embedding vector
//! completions embedding agent completions) chain their child
//! iterators rather than buffering.
//!
//! Tier blob writes (the six tier tables) are NOT yielded by the
//! streaming iterator — they're produced by separate `request_blob`
//! / `response_blob` helpers because they're written once per
//! lifecycle (request: on first chunk; response: also UPSERTed per
//! tick once we know the body, with the writer's shadow gating).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::completions::message::{
    File, ImageUrl, InputAudio, VideoUrl,
};

/// Every table in the `logs.*` schema. Matches `schema.sql` 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "LogTable")]
pub enum LogTable {
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

impl LogTable {
    /// Fully-qualified table name (`logs.<name>`). Used by the writer
    /// when composing `INSERT INTO …` statements.
    pub fn fq_name(self) -> &'static str {
        match self {
            LogTable::AgentCompletionRequests => "logs.agent_completion_requests",
            LogTable::AgentCompletionResponses => "logs.agent_completion_responses",
            LogTable::VectorCompletionRequests => "logs.vector_completion_requests",
            LogTable::VectorCompletionResponses => "logs.vector_completion_responses",
            LogTable::FunctionExecutionRequests => "logs.function_execution_requests",
            LogTable::FunctionExecutionResponses => "logs.function_execution_responses",
            LogTable::ToolResponse => "logs.tool_response",
            LogTable::AssistantResponseRefusal => "logs.assistant_response_refusal",
            LogTable::AssistantResponseReasoning => "logs.assistant_response_reasoning",
            LogTable::AssistantResponseToolCalls => "logs.assistant_response_tool_calls",
            LogTable::AssistantResponseContentText => "logs.assistant_response_content_text",
            LogTable::AssistantResponseContentImage => "logs.assistant_response_content_image",
            LogTable::AssistantResponseContentAudio => "logs.assistant_response_content_audio",
            LogTable::AssistantResponseContentVideo => "logs.assistant_response_content_video",
            LogTable::AssistantResponseContentFile => "logs.assistant_response_content_file",
            LogTable::ToolResponseContentText => "logs.tool_response_content_text",
            LogTable::ToolResponseContentImage => "logs.tool_response_content_image",
            LogTable::ToolResponseContentAudio => "logs.tool_response_content_audio",
            LogTable::ToolResponseContentVideo => "logs.tool_response_content_video",
            LogTable::ToolResponseContentFile => "logs.tool_response_content_file",
        }
    }
}

/// One streaming-content row to UPSERT. Variants are 1:1 with
/// [`LogTable`]'s streaming-content tables (the tier blob tables have
/// no `LogValue` variant — they're written through a separate path).
///
/// Borrowed: every variant lifts string / media payloads from the
/// owning chunk by reference so the iterator never copies content.
#[derive(Debug, Clone)]
pub enum LogValue<'a> {
    ToolResponse {
        response_id: &'a str,
        index: u64,
        tool_call_id: &'a str,
    },
    AssistantResponseRefusal {
        response_id: &'a str,
        index: u64,
        text: &'a str,
    },
    AssistantResponseReasoning {
        response_id: &'a str,
        index: u64,
        text: &'a str,
    },
    AssistantResponseToolCalls {
        response_id: &'a str,
        index: u64,
        tool_call_index: u64,
        tool_call_id: &'a str,
        arguments: &'a str,
    },

    AssistantResponseContentText {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        text: &'a str,
    },
    AssistantResponseContentImage {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        image_url: &'a ImageUrl,
    },
    AssistantResponseContentAudio {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        input_audio: &'a InputAudio,
    },
    AssistantResponseContentVideo {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        video_url: &'a VideoUrl,
        is_input: bool,
    },
    AssistantResponseContentFile {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        file: &'a File,
    },

    ToolResponseContentText {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        text: &'a str,
    },
    ToolResponseContentImage {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        image_url: &'a ImageUrl,
    },
    ToolResponseContentAudio {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        input_audio: &'a InputAudio,
    },
    ToolResponseContentVideo {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        video_url: &'a VideoUrl,
        is_input: bool,
    },
    ToolResponseContentFile {
        response_id: &'a str,
        index: u64,
        part_index: u64,
        file: &'a File,
    },
}

impl<'a> LogValue<'a> {
    /// The matching [`LogTable`] for this row.
    pub fn table(&self) -> LogTable {
        match self {
            LogValue::ToolResponse { .. } => LogTable::ToolResponse,
            LogValue::AssistantResponseRefusal { .. } => LogTable::AssistantResponseRefusal,
            LogValue::AssistantResponseReasoning { .. } => LogTable::AssistantResponseReasoning,
            LogValue::AssistantResponseToolCalls { .. } => LogTable::AssistantResponseToolCalls,
            LogValue::AssistantResponseContentText { .. } => LogTable::AssistantResponseContentText,
            LogValue::AssistantResponseContentImage { .. } => LogTable::AssistantResponseContentImage,
            LogValue::AssistantResponseContentAudio { .. } => LogTable::AssistantResponseContentAudio,
            LogValue::AssistantResponseContentVideo { .. } => LogTable::AssistantResponseContentVideo,
            LogValue::AssistantResponseContentFile { .. } => LogTable::AssistantResponseContentFile,
            LogValue::ToolResponseContentText { .. } => LogTable::ToolResponseContentText,
            LogValue::ToolResponseContentImage { .. } => LogTable::ToolResponseContentImage,
            LogValue::ToolResponseContentAudio { .. } => LogTable::ToolResponseContentAudio,
            LogValue::ToolResponseContentVideo { .. } => LogTable::ToolResponseContentVideo,
            LogValue::ToolResponseContentFile { .. } => LogTable::ToolResponseContentFile,
        }
    }
}

/// Iterator alias used at recursive boundaries (function execution →
/// vector completion → agent completion). Box-erased so the recursive
/// types can name themselves; one allocation per recursive descent,
/// not per row.
pub type LogRowIter<'a> = Box<dyn Iterator<Item = LogValue<'a>> + Send + 'a>;
