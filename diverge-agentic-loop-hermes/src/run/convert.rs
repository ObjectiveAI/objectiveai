//! The gateway's events, as the container's chunks.

use std::collections::VecDeque;

use diverge_provider_sdk::shared::containers::run_loop::response::{
    AgenticLoopChunk, AssistantReasoningChunk, AssistantTextContentChunk,
    AssistantToolCallChunk, NotificationChunk, ToolResponseChunk, UsageChunk,
};

use super::id;
use crate::response::Event;

/// One turn's conversion state.
///
/// The gateway's tool events carry no ids and no results, and come
/// in original call order — every `started` of a batch, then every
/// `completed`, in the same order. So a FIFO of open calls pairs
/// them: each completion pops the oldest start, and the pair IS the
/// id this container minted for it (`reports/parallel-tool-calls.md`).
/// A completion that matches no open start, or a different tool's,
/// is the degraded signature — a call abandoned at the gate — and
/// the queue is flushed rather than trusted further.
#[derive(Default)]
pub struct Turn {
    /// Open calls, oldest first.
    calls: VecDeque<Open>,
    /// Whether any text delta came: when none did, `run.completed`'s
    /// `output` is the assistant's only text and is spoken then.
    spoke: bool,
}

/// A tool call started and not yet completed.
struct Open {
    /// The id minted for it.
    id: String,
    /// The tool's name, for the consistency check.
    name: String,
}

impl Turn {
    /// The chunks one event becomes. Zero, one or several; the
    /// enqueue seam is not here — the runner applies it after a
    /// completion, by that completion's timestamp.
    pub fn convert(&mut self, event: Event) -> Vec<AgenticLoopChunk> {
        match event {
            Event::MessageDelta(delta) => {
                self.spoke = true;
                vec![text(delta.delta)]
            }
            Event::ReasoningAvailable(reasoning) => {
                vec![AgenticLoopChunk::AssistantReasoning(
                    AssistantReasoningChunk {
                        r#type: Default::default(),
                        parent_tool_call_id: None,
                        logprobs: None,
                        inner: rmcp::model::TextContent::new(reasoning.text),
                    },
                )]
            }
            Event::ToolStarted(started) => {
                let id = id::new();
                let name = started.tool.unwrap_or_default();
                self.calls.push_back(Open {
                    id: id.clone(),
                    name: name.clone(),
                });
                vec![AgenticLoopChunk::AssistantToolCall(AssistantToolCallChunk {
                    r#type: Default::default(),
                    parent_tool_call_id: None,
                    id,
                    meta: None,
                    name,
                    arguments: started.preview,
                })]
            }
            Event::ToolCompleted(completed) => {
                let name = completed.tool.unwrap_or_default();
                let mut chunks = Vec::new();
                let id = match self.calls.pop_front() {
                    Some(open) if open.name == name => open.id,
                    open => {
                        // The pairing broke: say so, drop what was
                        // open, and answer this completion under a
                        // fresh id rather than a wrong one.
                        self.calls.clear();
                        chunks.push(notification(
                            serde_json::json!({
                                "kind": "tool_pairing",
                                "completed": name,
                                "expected": open.map(|open| open.name),
                            }),
                            false,
                        ));
                        id::new()
                    }
                };
                let inner = if completed.error {
                    rmcp::model::CallToolResult::error(Vec::new())
                } else {
                    rmcp::model::CallToolResult::success(Vec::new())
                };
                chunks.push(AgenticLoopChunk::ToolResponse(ToolResponseChunk {
                    r#type: Default::default(),
                    parent_tool_call_id: None,
                    id,
                    inner,
                }));
                chunks
            }
            Event::RunCompleted(completed) => {
                let mut chunks = Vec::new();
                if !self.spoke && !completed.output.is_empty() {
                    chunks.push(text(completed.output));
                }
                chunks.push(AgenticLoopChunk::Usage(UsageChunk {
                    r#type: Default::default(),
                    completion_tokens: completed.usage.output_tokens as u64,
                    prompt_tokens: completed.usage.input_tokens as u64,
                    total_tokens: completed.usage.total_tokens as u64,
                    meta: None,
                }));
                chunks
            }
            Event::RunFailed(failed) => vec![notification(
                serde_json::json!({ "kind": "run", "error": failed.error }),
                true,
            )],
            Event::RunCancelled(_) => vec![notification(
                serde_json::json!({ "kind": "cancelled" }),
                true,
            )],
            Event::RunSteered(steered) => vec![notification(
                serde_json::json!({ "kind": "steered", "accepted": steered.accepted }),
                false,
            )],
            Event::ApprovalRequest(request) => vec![notification(
                serde_json::json!({
                    "kind": "approval",
                    "command": request.command,
                    "description": request.description,
                }),
                false,
            )],
            Event::ApprovalResponded(responded) => vec![notification(
                serde_json::json!({
                    "kind": "approval_responded",
                    "choice": responded.choice,
                }),
                false,
            )],
            Event::SubagentStart(start) => vec![notification(
                serde_json::json!({
                    "kind": "subagent_start",
                    "subagent_id": start.subagent_id,
                    "goal": start.goal,
                }),
                false,
            )],
            Event::SubagentComplete(complete) => vec![notification(
                serde_json::json!({
                    "kind": "subagent_complete",
                    "subagent_id": complete.subagent_id,
                    "status": complete.status,
                }),
                false,
            )],
            Event::Unknown(value) => vec![notification(
                serde_json::json!({ "kind": "unknown_event", "event": value }),
                false,
            )],
        }
    }
}

/// An assistant text chunk, on the main thread.
fn text(text: String) -> AgenticLoopChunk {
    AgenticLoopChunk::AssistantTextContent(AssistantTextContentChunk {
        r#type: Default::default(),
        parent_tool_call_id: None,
        logprobs: None,
        inner: rmcp::model::TextContent::new(text),
    })
}

/// A notification chunk, its fatality the caller's verdict.
pub fn notification(message: serde_json::Value, is_fatal: bool) -> AgenticLoopChunk {
    AgenticLoopChunk::Notification(NotificationChunk {
        r#type: Default::default(),
        is_fatal,
        message,
        meta: None,
    })
}
