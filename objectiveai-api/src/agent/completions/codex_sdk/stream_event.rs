//! Pure mapping from a [`super::ThreadEvent`] to a downstream
//! [`AgentCompletionChunk`]. Stateless — the streaming task in
//! `super::Client::create` tracks `thread_id`, `assistant_index`, and
//! the trailing usage chunk separately.
//!
//! Event mapping (matches the plan):
//! - `ThreadStarted` → `None` (consumed by client to track thread_id)
//! - `TurnStarted` → `None`
//! - `TurnCompleted { usage }` → empty assistant chunk with `usage`
//!   set and `finish_reason = stop`
//! - `TurnFailed` / `Error` → `Some(Err(Error::ThreadRun(...)))`
//! - `ItemStarted` / `ItemUpdated` → `None` (events are coarse;
//!   we wait for `ItemCompleted`)
//! - `ItemCompleted` with `AgentMessage` → assistant content chunk
//! - `ItemCompleted` with `Reasoning` → assistant reasoning chunk
//! - All other items / unknown variants → `None`

use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::agent::completions::response::streaming::{
    AgentCompletionChunk, AssistantResponseChunk, MessageChunk,
};
use objectiveai_sdk::agent::completions::response::{
    FinishReason, PromptTokensDetails, UpstreamUsage,
};
use objectiveai_sdk::agent::Upstream;

use super::{
    KnownThreadEvent, KnownThreadItem, ThreadEvent, ThreadItem, Usage,
};

/// Build the `UpstreamUsage` carried on the trailing assistant chunk
/// from a codex `Usage` payload.
fn upstream_usage(
    usage: &Usage,
    is_byok: bool,
    cost_multiplier: rust_decimal::Decimal,
) -> UpstreamUsage {
    let prompt_tokens = usage.input_tokens;
    let completion_tokens = usage.output_tokens;
    let total_tokens = prompt_tokens + completion_tokens;

    let prompt_tokens_details = Some(PromptTokensDetails {
        audio_tokens: None,
        cached_tokens: Some(usage.cached_input_tokens),
        cache_write_tokens: None,
        video_tokens: None,
    });

    // Codex CLI does not report a per-turn cost. We bill purely off
    // the cost_multiplier × token volume that downstream layers
    // compute; per-call cost is left as zero here. (BYOK gets a
    // negative cost via the same scheme so the wrapper credit
    // settles correctly.)
    let upstream_inference_cost = rust_decimal::Decimal::ZERO;
    let upstream_upstream_inference_cost = rust_decimal::Decimal::ZERO;
    let upstream_total_cost =
        upstream_inference_cost + upstream_upstream_inference_cost;
    let total_cost = upstream_total_cost * cost_multiplier;
    let (cost, cost_details, total_cost) = if is_byok {
        (
            total_cost - upstream_total_cost,
            Some(objectiveai_sdk::agent::completions::response::CostDetails {
                upstream_inference_cost,
                upstream_upstream_inference_cost,
            }),
            total_cost,
        )
    } else {
        (total_cost, None, total_cost)
    };

    UpstreamUsage {
        completion_tokens,
        prompt_tokens,
        total_tokens,
        completion_tokens_details: None,
        prompt_tokens_details,
        cost,
        cost_details,
        total_cost,
        cost_multiplier,
        is_byok,
    }
}

#[allow(clippy::too_many_arguments)]
fn assistant_chunk(
    id: String,
    created: u64,
    model: String,
    upstream: Upstream,
    upstream_id: String,
    assistant_index: u64,
    content: Option<RichContent>,
    reasoning: Option<String>,
    finish_reason: Option<FinishReason>,
    usage: Option<UpstreamUsage>,
    agent_instance_hierarchy: String,
    agent_id: String,
    agent_full_id: String,
    agent_remote: Option<objectiveai_sdk::RemotePath>,
) -> AgentCompletionChunk {
    AgentCompletionChunk {
        id,
        agent_instance_hierarchy,
        agent_id,
        agent_full_id,
        agent_remote,
        created,
        messages: vec![MessageChunk::Assistant(AssistantResponseChunk {
            index: assistant_index,
            created,
            model,
            upstream_id,
            reasoning,
            content,
            finish_reason,
            usage,
            ..Default::default()
        })],
        object: Default::default(),
        usage: None,
        upstream,
        error: None,
        continuation: None,
        messages_queued: None,
    }
}

/// Map one `ThreadEvent` to an optional downstream chunk.
///
/// `thread_id` is the latest seen `thread.started` id (empty if none yet).
/// Returns `Ok(None)` for events that produce no downstream chunk.
#[allow(clippy::too_many_arguments)]
pub fn into_downstream(
    event: ThreadEvent,
    id: String,
    created: u64,
    model: String,
    assistant_index: u64,
    is_byok: bool,
    cost_multiplier: rust_decimal::Decimal,
    upstream: Upstream,
    thread_id: &str,
    agent_instance_hierarchy: String,
    agent_id: String,
    agent_full_id: String,
    agent_remote: Option<objectiveai_sdk::RemotePath>,
) -> Option<Result<AgentCompletionChunk, super::Error>> {
    let known = match event {
        ThreadEvent::Known(k) => k,
        ThreadEvent::Unknown(_) => return None,
    };

    match known {
        KnownThreadEvent::ThreadStarted(_) | KnownThreadEvent::TurnStarted(_) => None,

        KnownThreadEvent::TurnCompleted(c) => {
            let usage = upstream_usage(&c.usage, is_byok, cost_multiplier);
            Some(Ok(assistant_chunk(
                id,
                created,
                model,
                upstream,
                thread_id.to_string(),
                assistant_index,
                None,
                None,
                Some(FinishReason::Stop),
                Some(usage),
                agent_instance_hierarchy,
                agent_id,
                agent_full_id,
                agent_remote,
            )))
        }

        KnownThreadEvent::TurnFailed(f) => {
            Some(Err(super::Error::ThreadRun(f.error.message)))
        }
        KnownThreadEvent::Error(e) => {
            Some(Err(super::Error::ThreadRun(e.message)))
        }

        KnownThreadEvent::ItemStarted(_) | KnownThreadEvent::ItemUpdated(_) => None,

        KnownThreadEvent::ItemCompleted(ic) => {
            let known = match ic.item {
                ThreadItem::Known(k) => k,
                ThreadItem::Unknown(_) => return None,
            };
            match known {
                KnownThreadItem::AgentMessage(m) => Some(Ok(assistant_chunk(
                    id,
                    created,
                    model,
                    upstream,
                    thread_id.to_string(),
                    assistant_index,
                    Some(RichContent::Text(m.text)),
                    None,
                    None,
                    None,
                    agent_instance_hierarchy,
                    agent_id,
                    agent_full_id,
                    agent_remote,
                ))),
                KnownThreadItem::Reasoning(r) => Some(Ok(assistant_chunk(
                    id,
                    created,
                    model,
                    upstream,
                    thread_id.to_string(),
                    assistant_index,
                    None,
                    Some(r.text),
                    None,
                    None,
                    agent_instance_hierarchy,
                    agent_id,
                    agent_full_id,
                    agent_remote,
                ))),
                // Sandboxed read-only / temp-cwd posture means file
                // changes / command execution don't actually mutate
                // anything; surface nothing for them. MCP / web search
                // / todo lists are also informational at this layer.
                KnownThreadItem::CommandExecution(_)
                | KnownThreadItem::FileChange(_)
                | KnownThreadItem::McpToolCall(_)
                | KnownThreadItem::WebSearch(_)
                | KnownThreadItem::TodoList(_)
                | KnownThreadItem::Error(_) => None,
            }
        }
    }
}
