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

use objectiveai_sdk::agent::completions::message::{
    AssistantToolCall, Message, RichContent, RichContentPart,
};
use objectiveai_sdk::agent::completions::response::ToolResponse;
use objectiveai_sdk::agent::completions::response::streaming::{
    AgentCompletionChunk, AssistantResponseChunk, MessageChunk,
};
use objectiveai_sdk::functions::executions::response::streaming::{
    FunctionExecutionChunk, TaskChunk, VectorCompletionTaskChunk,
};
use objectiveai_sdk::vector::completions::response::Vote;
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
    // The completion's own in-band error, when set (lazy-set,
    // cumulative — present on every subsequent chunk once set; the
    // writer dedupes per (aih, response_id)).
    let error = chunk.error.as_ref().map(move |error| WriterItem::Error {
        agent_instance_hierarchy: agent_hierarchy,
        response_id,
        error,
    });
    let rows = chunk
        .messages
        .iter()
        .flat_map(move |msg| message_chunk_rows(response_id, agent_hierarchy, msg))
        .map(WriterItem::Row);
    Box::new(usage.into_iter().chain(error).chain(rows))
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

/// One nested agent completion's liveness, borrowed from the folded
/// cumulative chunk. `finished` = usage arrived (present only on a
/// completion's final chunk); `errored` = the completion carries its
/// own in-band error (already persisted via [`WriterItem::Error`]).
/// The mid-stream failure sweep logs the stream error for every
/// completion that is neither.
pub struct CompletionStatus<'a> {
    pub agent_instance_hierarchy: &'a str,
    pub response_id: &'a str,
    pub finished: bool,
    pub errored: bool,
}

/// Boxed iterator of [`CompletionStatus`]es.
pub type CompletionStatuses<'a> = Box<dyn Iterator<Item = CompletionStatus<'a>> + Send + 'a>;

/// Status of the one completion an agent chunk IS.
pub fn agent_completion_statuses<'a>(
    chunk: &'a AgentCompletionChunk,
) -> CompletionStatuses<'a> {
    Box::new(std::iter::once(CompletionStatus {
        agent_instance_hierarchy: chunk.agent_instance_hierarchy.as_str(),
        response_id: chunk.id.as_str(),
        finished: chunk.usage.is_some(),
        errored: chunk.error.is_some(),
    }))
}

/// Statuses of every embedded per-agent completion in a vector chunk.
pub fn vector_completion_statuses<'a>(
    chunk: &'a VectorCompletionChunk,
) -> CompletionStatuses<'a> {
    Box::new(
        chunk
            .completions
            .iter()
            .flat_map(|c| agent_completion_statuses(&c.inner)),
    )
}

/// Statuses of every nested agent completion in a function chunk —
/// recursive (function tasks chain back in), reasoning summaries
/// included. Mirrors [`function_execution_chunk_rows`]'s traversal.
pub fn function_execution_statuses<'a>(
    chunk: &'a FunctionExecutionChunk,
) -> CompletionStatuses<'a> {
    let tasks = chunk.tasks.iter().flat_map(|task| match task {
        TaskChunk::FunctionExecution(wrapper) => {
            function_execution_statuses(&wrapper.inner)
        }
        TaskChunk::VectorCompletion(wrapper) => {
            vector_completion_statuses(&wrapper.inner)
        }
    });
    let reasoning = chunk
        .reasoning
        .iter()
        .flat_map(|r| agent_completion_statuses(&r.inner));
    Box::new(tasks.chain(reasoning))
}

// ---- internal helpers -------------------------------------------------

fn task_chunk_rows<'a>(task: &'a TaskChunk) -> WriterItems<'a> {
    match task {
        TaskChunk::FunctionExecution(wrapper) => {
            function_execution_chunk_rows(&wrapper.inner)
        }
        TaskChunk::VectorCompletion(wrapper) => vector_task_rows(wrapper),
    }
}

/// A function-execution vector-completion task: for each per-agent
/// completion, emit — in order — the task's request messages, the
/// choices (with this agent's inline voting key), the agent's own
/// response rows, then this agent's vote (closer). Every emitted row
/// uses the per-agent AGENT-COMPLETION `response_id`
/// (`c.inner.id`) — never the vector/task id — so `read all` groups
/// them under the agent completion.
fn vector_task_rows<'a>(
    wrapper: &'a VectorCompletionTaskChunk,
) -> WriterItems<'a> {
    let vc = &wrapper.inner;
    let request_messages = wrapper.request_messages.as_deref();
    let request_choices = wrapper.request_choices.as_deref();
    Box::new(vc.completions.iter().flat_map(move |c| {
        let agent = &c.inner;
        let response_id = agent.id.as_str();
        let aih = agent.agent_instance_hierarchy.as_str();

        let req_msgs: RowsIter<'a> = match request_messages {
            Some(msgs) => request_message_rows(response_id, aih, msgs),
            None => Box::new(std::iter::empty()),
        };
        let choices: RowsIter<'a> =
            match (request_choices, c.request_choice_keys.as_deref()) {
                (Some(chs), Some(keys)) => {
                    vector_choice_rows(response_id, aih, chs, keys)
                }
                _ => Box::new(std::iter::empty()),
            };
        let vote = vote_row(response_id, aih, &vc.votes, c.index)
            .map(WriterItem::Row);

        Box::new(
            req_msgs
                .map(WriterItem::Row)
                .chain(choices.map(WriterItem::Row))
                .chain(agent_completion_chunk_rows(agent))
                .chain(vote),
        ) as WriterItems<'a>
    }))
}

/// This agent's vote for the task, matched by `completion_index` —
/// which equals the vector wrapper's `index`. This is an exact match
/// even when swarm `count > 1` gives several completions the same
/// `agent_id`.
fn vote_row<'a>(
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    votes: &'a [Vote],
    completion_index: u64,
) -> Option<RowValue<'a>> {
    votes
        .iter()
        .find(|v| v.completion_index == Some(completion_index))
        .map(|v| RowValue::ResponseVectorVote {
            response_id,
            agent_instance_hierarchy,
            vote: v.vote.as_slice(),
        })
}

/// One choice per entry in `choices`, keyed by choice index: the head
/// row (this agent's `keys[i]`) then the choice's content parts.
fn vector_choice_rows<'a>(
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    choices: &'a [RichContent],
    keys: &'a [String],
) -> RowsIter<'a> {
    Box::new(choices.iter().enumerate().flat_map(move |(i, content)| {
        let choice_index = i as u64;
        let key = keys.get(i).map(|k| k.as_str()).unwrap_or("");
        let head = std::iter::once(RowValue::RequestVectorChoice {
            response_id,
            agent_instance_hierarchy,
            choice_index,
            key,
        });
        Box::new(head.chain(request_content_rows(
            RequestTarget::VectorChoice,
            response_id,
            agent_instance_hierarchy,
            choice_index,
            content,
        ))) as RowsIter<'a>
    }))
}

/// Walk a request/task `Vec<Message>` into request_message rows,
/// paralleling the response walk but into the `request_message_*`
/// tables. `index` is the message's position in the array. Shared by
/// the function-tier task walk (from `task.request_messages`) and the
/// agent-tier writer (from the request body).
pub(super) fn request_message_rows<'a>(
    response_id: &'a str,
    aih: &'a str,
    messages: &'a [Message],
) -> RowsIter<'a> {
    Box::new(messages.iter().enumerate().flat_map(move |(i, msg)| {
        let index = i as u64;
        let out: RowsIter<'a> = match msg {
            Message::User(u) => {
                request_content_rows(RequestTarget::User, response_id, aih, index, &u.content)
            }
            Message::Assistant(a) => {
                // reasoning → tool_calls → content → refusal
                let reasoning = a.reasoning.iter().map(move |t| {
                    RowValue::RequestMessageAssistantReasoning {
                        response_id,
                        agent_instance_hierarchy: aih,
                        index,
                        text: t.as_str(),
                    }
                });
                let tool_calls =
                    a.tool_calls.iter().flatten().enumerate().map(move |(tci, tc)| {
                        let AssistantToolCall::Function { id, function } = tc;
                        RowValue::RequestMessageAssistantToolCalls {
                            response_id,
                            agent_instance_hierarchy: aih,
                            index,
                            tool_call_index: tci as u64,
                            tool_call_id: id.as_str(),
                            function_name: function.name.as_str(),
                            arguments: function.arguments.as_str(),
                        }
                    });
                let content = a.content.iter().flat_map(move |c| {
                    request_content_rows(RequestTarget::Assistant, response_id, aih, index, c)
                });
                let refusal = a.refusal.iter().map(move |t| {
                    RowValue::RequestMessageAssistantRefusal {
                        response_id,
                        agent_instance_hierarchy: aih,
                        index,
                        text: t.as_str(),
                    }
                });
                Box::new(reasoning.chain(tool_calls).chain(content).chain(refusal))
            }
            Message::Tool(t) => {
                let head = std::iter::once(RowValue::RequestMessageTool {
                    response_id,
                    agent_instance_hierarchy: aih,
                    index,
                    tool_call_id: t.tool_call_id.as_str(),
                });
                Box::new(head.chain(request_content_rows(
                    RequestTarget::Tool,
                    response_id,
                    aih,
                    index,
                    &t.content,
                )))
            }
        };
        out
    }))
}

/// Which request content table group a content part targets.
#[derive(Debug, Clone, Copy)]
enum RequestTarget {
    User,
    Assistant,
    Tool,
    VectorChoice,
}

fn request_content_rows<'a>(
    target: RequestTarget,
    response_id: &'a str,
    aih: &'a str,
    index: u64,
    content: &'a RichContent,
) -> RowsIter<'a> {
    match content {
        RichContent::Text(text) => Box::new(std::iter::once(request_content_text(
            target, response_id, aih, index, 0, text.as_str(),
        ))),
        RichContent::Parts(parts) => {
            Box::new(parts.iter().enumerate().map(move |(pi, part)| {
                request_content_part(target, response_id, aih, index, pi as u64, part)
            }))
        }
    }
}

fn request_content_text<'a>(
    target: RequestTarget,
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    index: u64,
    part_index: u64,
    text: &'a str,
) -> RowValue<'a> {
    match target {
        RequestTarget::User => RowValue::RequestMessageUserContentText { response_id, agent_instance_hierarchy, index, part_index, text },
        RequestTarget::Assistant => RowValue::RequestMessageAssistantContentText { response_id, agent_instance_hierarchy, index, part_index, text },
        RequestTarget::Tool => RowValue::RequestMessageToolContentText { response_id, agent_instance_hierarchy, index, part_index, text },
        RequestTarget::VectorChoice => RowValue::RequestVectorChoiceContentText { response_id, agent_instance_hierarchy, choice_index: index, part_index, text },
    }
}

fn request_content_part<'a>(
    target: RequestTarget,
    response_id: &'a str,
    agent_instance_hierarchy: &'a str,
    index: u64,
    part_index: u64,
    part: &'a RichContentPart,
) -> RowValue<'a> {
    match part {
        RichContentPart::Text { text } => {
            request_content_text(target, response_id, agent_instance_hierarchy, index, part_index, text.as_str())
        }
        RichContentPart::ImageUrl { image_url } => match target {
            RequestTarget::User => RowValue::RequestMessageUserContentImage { response_id, agent_instance_hierarchy, index, part_index, image_url },
            RequestTarget::Assistant => RowValue::RequestMessageAssistantContentImage { response_id, agent_instance_hierarchy, index, part_index, image_url },
            RequestTarget::Tool => RowValue::RequestMessageToolContentImage { response_id, agent_instance_hierarchy, index, part_index, image_url },
            RequestTarget::VectorChoice => RowValue::RequestVectorChoiceContentImage { response_id, agent_instance_hierarchy, choice_index: index, part_index, image_url },
        },
        RichContentPart::InputAudio { input_audio } => match target {
            RequestTarget::User => RowValue::RequestMessageUserContentAudio { response_id, agent_instance_hierarchy, index, part_index, input_audio },
            RequestTarget::Assistant => RowValue::RequestMessageAssistantContentAudio { response_id, agent_instance_hierarchy, index, part_index, input_audio },
            RequestTarget::Tool => RowValue::RequestMessageToolContentAudio { response_id, agent_instance_hierarchy, index, part_index, input_audio },
            RequestTarget::VectorChoice => RowValue::RequestVectorChoiceContentAudio { response_id, agent_instance_hierarchy, choice_index: index, part_index, input_audio },
        },
        RichContentPart::InputVideo { video_url } | RichContentPart::VideoUrl { video_url } => match target {
            RequestTarget::User => RowValue::RequestMessageUserContentVideo { response_id, agent_instance_hierarchy, index, part_index, video_url },
            RequestTarget::Assistant => RowValue::RequestMessageAssistantContentVideo { response_id, agent_instance_hierarchy, index, part_index, video_url },
            RequestTarget::Tool => RowValue::RequestMessageToolContentVideo { response_id, agent_instance_hierarchy, index, part_index, video_url },
            RequestTarget::VectorChoice => RowValue::RequestVectorChoiceContentVideo { response_id, agent_instance_hierarchy, choice_index: index, part_index, video_url },
        },
        RichContentPart::File { file } => match target {
            RequestTarget::User => RowValue::RequestMessageUserContentFile { response_id, agent_instance_hierarchy, index, part_index, file },
            RequestTarget::Assistant => RowValue::RequestMessageAssistantContentFile { response_id, agent_instance_hierarchy, index, part_index, file },
            RequestTarget::Tool => RowValue::RequestMessageToolContentFile { response_id, agent_instance_hierarchy, index, part_index, file },
            RequestTarget::VectorChoice => RowValue::RequestVectorChoiceContentFile { response_id, agent_instance_hierarchy, choice_index: index, part_index, file },
        },
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
                WriterItem::Row(_) | WriterItem::Error { .. } => None,
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
