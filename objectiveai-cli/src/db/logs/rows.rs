//! Free functions that walk a chunk and yield every streaming-content
//! [`RowValue`] it implies. One entry point per top-level chunk type:
//!
//! - [`agent_completion_chunk_rows`]
//! - [`vector_completion_chunk_rows`]
//! - [`function_execution_chunk_rows`]
//!
//! The function-execution walker is recursive — it forwards into the
//! vector walker (and back into itself for nested function tasks),
//! which in turn forwards into the agent walker. No collection
//! happens: each yielded row borrows from the input chunk and the
//! writer drains the iterator one element at a time.
//!
//! Every yielded `RowValue` carries the enclosing agent-completion
//! chunk's `response_id` AND `agent_instance_hierarchy`, so the
//! writer can populate `logs.messages` / `logs.messages_queue`
//! without a side-channel.

use objectiveai_sdk::agent::completions::message::{RichContent, RichContentPart};
use objectiveai_sdk::agent::completions::response::ToolResponse;
use objectiveai_sdk::agent::completions::response::streaming::{
    AgentCompletionChunk, AssistantResponseChunk, MessageChunk,
};
use objectiveai_sdk::functions::executions::response::streaming::{
    FunctionExecutionChunk, TaskChunk,
};
use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk;

use super::row::{RowValue, RowsIter};

/// Entry: walk an agent-completion chunk's `messages` and yield every
/// streaming-content row keyed by the chunk's own `id` and
/// `agent_instance_hierarchy`.
pub fn agent_completion_chunk_rows<'a>(
    chunk: &'a AgentCompletionChunk,
) -> RowsIter<'a> {
    let response_id = chunk.id.as_str();
    let agent_hierarchy = chunk.agent_instance_hierarchy.as_str();
    Box::new(
        chunk
            .messages
            .iter()
            .flat_map(move |msg| message_chunk_rows(response_id, agent_hierarchy, msg)),
    )
}

/// Entry: walk every embedded per-agent completion in a vector chunk
/// and forward to [`agent_completion_chunk_rows`].
pub fn vector_completion_chunk_rows<'a>(
    chunk: &'a VectorCompletionChunk,
) -> RowsIter<'a> {
    Box::new(
        chunk
            .completions
            .iter()
            .flat_map(|c| agent_completion_chunk_rows(&c.inner)),
    )
}

/// Entry: walk every task in a function chunk and forward to the
/// matching tier walker. Reasoning summary's inner agent completion
/// also flows through. Recursive: function tasks chain back into this
/// function.
pub fn function_execution_chunk_rows<'a>(
    chunk: &'a FunctionExecutionChunk,
) -> RowsIter<'a> {
    let task_iter = chunk
        .tasks
        .iter()
        .flat_map(|task| task_chunk_rows(task));
    let reasoning_iter = chunk
        .reasoning
        .iter()
        .flat_map(|r| agent_completion_chunk_rows(&r.inner));
    Box::new(task_iter.chain(reasoning_iter))
}

// ---- internal helpers -------------------------------------------------

fn task_chunk_rows<'a>(task: &'a TaskChunk) -> RowsIter<'a> {
    match task {
        TaskChunk::FunctionExecution(wrapper) => {
            function_execution_chunk_rows(&wrapper.inner)
        }
        TaskChunk::VectorCompletion(wrapper) => {
            vector_completion_chunk_rows(&wrapper.inner)
        }
    }
}

fn message_chunk_rows<'a>(
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    msg: &'a MessageChunk,
) -> RowsIter<'a> {
    match msg {
        MessageChunk::Assistant(a) => {
            assistant_response_chunk_rows(response_id, agent_instance_hierarchy, a)
        }
        MessageChunk::Tool(t) => {
            tool_response_rows(response_id, agent_instance_hierarchy, t)
        }
    }
}

fn assistant_response_chunk_rows<'a>(
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    chunk: &'a AssistantResponseChunk,
) -> RowsIter<'a> {
    let index = chunk.index;

    // Prepend: `MessageQueueContent` rows for every consumed
    // `message_queue_contents.id` the API stamped. Yielded ahead
    // of the message body so `logs.messages` chronicles
    // consumption before the body the agent produced from it.
    let message_queue_iter = chunk
        .request_message_ids
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .copied()
        .map(move |message_queue_content_id| RowValue::MessageQueueContent {
            response_id,
            agent_instance_hierarchy,
            message_queue_content_id,
        });

    // Emission order: reasoning → tool_calls → content → refusal.
    // Refusal goes last on purpose — when a model refuses it's
    // typically the terminal signal of the turn, so readers see all
    // the actual work (reasoning / tool calls / content parts) before
    // the refusal stamp.
    let reasoning_iter = chunk.reasoning.iter().map(move |text| {
        RowValue::AssistantResponseReasoning {
            response_id,
            agent_instance_hierarchy,
            index,
            text: text.as_str(),
        }
    });
    let tool_calls_iter = chunk
        .tool_calls
        .iter()
        .flatten()
        .enumerate()
        .filter_map(move |(tc_idx, tc)| {
            let id = tc.id.as_deref()?;
            let args = tc.function.as_ref().and_then(|f| f.arguments.as_deref())?;
            Some(RowValue::AssistantResponseToolCalls {
                response_id,
                agent_instance_hierarchy,
                index,
                tool_call_index: tc_idx as u64,
                tool_call_id: id,
                arguments: args,
            })
        });
    let content_iter = chunk
        .content
        .iter()
        .flat_map(move |c| assistant_content_rows(response_id, agent_instance_hierarchy, index, c));
    let refusal_iter = chunk.refusal.iter().map(move |text| {
        RowValue::AssistantResponseRefusal {
            response_id,
            agent_instance_hierarchy,
            index,
            text: text.as_str(),
        }
    });

    Box::new(
        message_queue_iter
            .chain(reasoning_iter)
            .chain(tool_calls_iter)
            .chain(content_iter)
            .chain(refusal_iter),
    )
}

fn tool_response_rows<'a>(
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    response: &'a ToolResponse,
) -> RowsIter<'a> {
    let index = response.index;
    // Same MessageQueueContent prepend as the assistant path —
    // surfaced for wire-shape symmetry. Currently the API never
    // populates `ToolResponse.request_message_ids`, so this iter
    // is empty in practice.
    let message_queue_iter = response
        .request_message_ids
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .copied()
        .map(move |message_queue_content_id| RowValue::MessageQueueContent {
            response_id,
            agent_instance_hierarchy,
            message_queue_content_id,
        });
    let head = std::iter::once(RowValue::ToolResponse {
        response_id,
        agent_instance_hierarchy,
        index,
        tool_call_id: response.inner.tool_call_id.as_str(),
    });
    Box::new(
        message_queue_iter.chain(head).chain(tool_content_rows(
            response_id,
            agent_instance_hierarchy,
            index,
            &response.inner.content,
        )),
    )
}

fn assistant_content_rows<'a>(
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    index: u64,
    content: &'a RichContent,
) -> RowsIter<'a> {
    match content {
        RichContent::Text(text) => Box::new(std::iter::once(RowValue::AssistantResponseContentText {
            response_id,
            agent_instance_hierarchy,
            index,
            part_index: 0,
            text: text.as_str(),
        })),
        RichContent::Parts(parts) => Box::new(parts.iter().enumerate().map(move |(part_index, part)| {
            assistant_content_part(response_id, agent_instance_hierarchy, index, part_index as u64, part)
        })),
    }
}

fn assistant_content_part<'a>(
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    index: u64,
    part_index: u64,
    part: &'a RichContentPart,
) -> RowValue<'a> {
    match part {
        RichContentPart::Text { text } => RowValue::AssistantResponseContentText {
            response_id, agent_instance_hierarchy, index, part_index, text: text.as_str(),
        },
        RichContentPart::ImageUrl { image_url } => RowValue::AssistantResponseContentImage {
            response_id, agent_instance_hierarchy, index, part_index, image_url,
        },
        RichContentPart::InputAudio { input_audio } => RowValue::AssistantResponseContentAudio {
            response_id, agent_instance_hierarchy, index, part_index, input_audio,
        },
        RichContentPart::InputVideo { video_url }
        | RichContentPart::VideoUrl { video_url } => RowValue::AssistantResponseContentVideo {
            response_id, agent_instance_hierarchy, index, part_index, video_url,
        },
        RichContentPart::File { file } => RowValue::AssistantResponseContentFile {
            response_id, agent_instance_hierarchy, index, part_index, file,
        },
    }
}

fn tool_content_rows<'a>(
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    index: u64,
    content: &'a RichContent,
) -> RowsIter<'a> {
    match content {
        RichContent::Text(text) => Box::new(std::iter::once(RowValue::ToolResponseContentText {
            response_id,
            agent_instance_hierarchy,
            index,
            part_index: 0,
            text: text.as_str(),
        })),
        RichContent::Parts(parts) => Box::new(parts.iter().enumerate().map(move |(part_index, part)| {
            tool_content_part(response_id, agent_instance_hierarchy, index, part_index as u64, part)
        })),
    }
}

fn tool_content_part<'a>(
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    index: u64,
    part_index: u64,
    part: &'a RichContentPart,
) -> RowValue<'a> {
    match part {
        RichContentPart::Text { text } => RowValue::ToolResponseContentText {
            response_id, agent_instance_hierarchy, index, part_index, text: text.as_str(),
        },
        RichContentPart::ImageUrl { image_url } => RowValue::ToolResponseContentImage {
            response_id, agent_instance_hierarchy, index, part_index, image_url,
        },
        RichContentPart::InputAudio { input_audio } => RowValue::ToolResponseContentAudio {
            response_id, agent_instance_hierarchy, index, part_index, input_audio,
        },
        RichContentPart::InputVideo { video_url }
        | RichContentPart::VideoUrl { video_url } => RowValue::ToolResponseContentVideo {
            response_id, agent_instance_hierarchy, index, part_index, video_url,
        },
        RichContentPart::File { file } => RowValue::ToolResponseContentFile {
            response_id, agent_instance_hierarchy, index, part_index, file,
        },
    }
}
