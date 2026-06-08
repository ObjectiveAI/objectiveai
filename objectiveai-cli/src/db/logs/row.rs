//! Row-shape types: [`RowTable`] (closed enum of every table in the
//! `logs.*` schema) and [`RowValue<'a>`] (borrowed sum type of every
//! row body the writer can be asked to INSERT or UPDATE). One
//! [`RowValue`] variant per streaming-content table. Tier-blob writes
//! don't go through this enum — they're produced separately because
//! they're written once per stream lifecycle.

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

impl RowTable {
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

/// One streaming-content row to INSERT or UPDATE. Variants are 1:1
/// with [`RowTable`]'s streaming-content tables. Borrowed: every
/// variant lifts string / media payloads from the owning chunk by
/// reference so the iterator never copies content.
#[derive(Debug, Clone)]
pub enum RowValue<'a> {
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
}

/// Boxed-iterator alias used at the recursive boundaries
/// (function execution → vector completion → agent completion).
/// One Box per recursive descent, never per leaf row.
pub type RowsIter<'a> = Box<dyn Iterator<Item = RowValue<'a>> + Send + 'a>;
