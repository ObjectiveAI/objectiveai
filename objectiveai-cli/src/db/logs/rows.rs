//! Free functions that walk a chunk and yield every [`WriterItem`] it
//! implies — the streaming-content [`RowValue`]s plus, at each nested
//! agent completion carrying usage, a [`WriterItem::Usage`] token-count
//! snapshot. One entry point per top-level chunk type:
//!
//! - [`agent_completion_chunk_rows`]
//! - [`vector_completion_chunk_rows`]
//! - [`function_execution_chunk_rows`]
//!
//! The function-execution walker is recursive — it forwards into the
//! vector walker (and back into itself for nested function tasks),
//! which in turn forwards into the agent walker. No collection
//! happens: each yielded item borrows from the input chunk and the
//! writer drains the iterator one element at a time — a single
//! traversal covers both rows and usage.
//!
//! Every yielded `RowValue` carries the enclosing agent-completion
//! chunk's `response_id` AND `agent_instance_hierarchy`, so the
//! writer can populate `objectiveai.messages` / `objectiveai.messages_queue`
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

use super::row::{RowValue, RowsIter, WriterItem, WriterItems};

/// Entry: walk an agent-completion chunk. Emits a [`WriterItem::Usage`]
/// first when the chunk carries a non-`None` usage (its `total_tokens`
/// snapshot), then every streaming-content row keyed by the chunk's own
/// `id` and `agent_instance_hierarchy`.
pub fn agent_completion_chunk_rows<'a>(
    chunk: &'a AgentCompletionChunk,
) -> WriterItems<'a> {
    let response_id = chunk.id.as_str();
    let agent_hierarchy = chunk.agent_instance_hierarchy.as_str();
    let usage = chunk.usage.as_ref().map(move |u| WriterItem::Usage {
        agent_instance_hierarchy: agent_hierarchy,
        total_tokens: u.total_tokens,
    });
    let rows = chunk
        .messages
        .iter()
        .flat_map(move |msg| message_chunk_rows(response_id, agent_hierarchy, msg))
        .map(WriterItem::Row);
    Box::new(usage.into_iter().chain(rows))
}

/// Entry: walk every embedded per-agent completion in a vector chunk
/// and forward to [`agent_completion_chunk_rows`] (usage included).
pub fn vector_completion_chunk_rows<'a>(
    chunk: &'a VectorCompletionChunk,
) -> WriterItems<'a> {
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
/// function. Usage items interleave with rows via the leaf walker.
pub fn function_execution_chunk_rows<'a>(
    chunk: &'a FunctionExecutionChunk,
) -> WriterItems<'a> {
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

fn task_chunk_rows<'a>(task: &'a TaskChunk) -> WriterItems<'a> {
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
    // of the message body so `objectiveai.messages` chronicles
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
        .filter_map(move |tc| {
            let id = tc.id.as_deref()?;
            let name = tc.function.as_ref().and_then(|f| f.name.as_deref())?;
            let args = tc.function.as_ref().and_then(|f| f.arguments.as_deref())?;
            Some(RowValue::AssistantResponseToolCalls {
                response_id,
                agent_instance_hierarchy,
                index,
                // The tool call's own wire `index`, NOT its position in
                // the merged Vec. `push` correlates streamed tool-call
                // deltas by `index`, so a call whose wire index is e.g.
                // 2 can sit at Vec position 0 (or move as earlier-index
                // deltas arrive). The row PK is
                // (response_id, index, tool_call_index) — keying it on
                // Vec position would mislabel calls and let the shadow
                // key drift across chunks; the wire index is stable.
                tool_call_index: tc.index,
                tool_call_id: id,
                function_name: name,
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

#[cfg(test)]
mod tests {
    use super::*;
    use objectiveai_sdk::agent::completions::response::Usage;

    fn agent_chunk(aih: &str, total: Option<u64>) -> AgentCompletionChunk {
        AgentCompletionChunk {
            agent_instance_hierarchy: aih.to_string(),
            usage: total.map(|t| Usage { total_tokens: t, ..Default::default() }),
            ..Default::default()
        }
    }

    /// Collect just the `Usage` items from a walk.
    fn usages<'a>(items: WriterItems<'a>) -> Vec<(&'a str, u64)> {
        items
            .filter_map(|item| match item {
                WriterItem::Usage { agent_instance_hierarchy, total_tokens } => {
                    Some((agent_instance_hierarchy, total_tokens))
                }
                WriterItem::Row(_) => None,
            })
            .collect()
    }

    #[test]
    fn agent_walk_emits_usage_only_when_present() {
        assert_eq!(
            usages(agent_completion_chunk_rows(&agent_chunk("a/b", Some(42)))),
            vec![("a/b", 42)]
        );
        assert!(usages(agent_completion_chunk_rows(&agent_chunk("a/b", None))).is_empty());
    }

    #[test]
    fn vector_walk_surfaces_nested_usage_and_skips_none() {
        use objectiveai_sdk::vector::completions::response::streaming::AgentCompletionChunk as VecAgent;
        let chunk = VectorCompletionChunk {
            completions: vec![
                VecAgent { index: 0, inner: agent_chunk("a/x", Some(10)), ..Default::default() },
                VecAgent { index: 1, inner: agent_chunk("a/y", None), ..Default::default() },
                VecAgent { index: 2, inner: agent_chunk("a/z", Some(7)), ..Default::default() },
            ],
            ..Default::default()
        };
        assert_eq!(
            usages(vector_completion_chunk_rows(&chunk)),
            vec![("a/x", 10), ("a/z", 7)]
        );
    }
}
