//! Pure mapping from a [`super::GeminiEvent`] to a downstream
//! [`AgentCompletionChunk`]. Stateless — the streaming task in
//! `super::Client::create` tracks the assistant index and the trailing
//! usage chunk separately.
//!
//! Event mapping:
//! - `text`        → assistant content delta chunk.
//! - `thinking`    → assistant reasoning delta chunk.
//! - `tool_use`    → assistant chunk with a single `tool_calls` entry
//!   (informational — the gemini runner dispatched it internally; the
//!   API does NOT re-dispatch).
//! - `tool_result` → a `Tool` MessageChunk carrying the result.
//! - `usage`       → trailing assistant chunk with `usage` set and
//!   `finish_reason = stop`.
//! - `Unknown`     → `None` (forward-compatible).
//!
//! Index discipline (so the orchestrator's tool-call extractor — which
//! only inspects the LAST assistant message — terminates after this
//! single runner turn): each `tool_use` chunk lands at the current
//! assistant index, and the following `tool_result` `Tool` chunk
//! "finishes" that index (via `chunk_finishes_assistant`), bumping the
//! caller's index. The trailing `usage`/text chunks therefore land on a
//! fresh assistant index that carries no `tool_calls`, so the
//! orchestrator finds nothing callable and stops.

use objectiveai_sdk::agent::completions::message::{
    AssistantToolCallDelta, AssistantToolCallFunctionDelta,
    AssistantToolCallType, RichContent, ToolMessage,
};
use objectiveai_sdk::agent::completions::response::streaming::{
    AgentCompletionChunk, AssistantResponseChunk, MessageChunk,
};
use objectiveai_sdk::agent::completions::response::{
    FinishReason, PromptTokensDetails, ToolResponse, UpstreamUsage,
};
use objectiveai_sdk::agent::Upstream;

use super::GeminiEvent;

/// Build the `UpstreamUsage` carried on the trailing assistant chunk
/// from a gemini `usage` event.
fn upstream_usage(
    input_tokens: u64,
    output_tokens: u64,
    is_byok: bool,
    cost_multiplier: rust_decimal::Decimal,
) -> UpstreamUsage {
    let prompt_tokens = input_tokens;
    let completion_tokens = output_tokens;
    let total_tokens = prompt_tokens + completion_tokens;

    let prompt_tokens_details = Some(PromptTokensDetails {
        audio_tokens: None,
        cached_tokens: None,
        cache_write_tokens: None,
        video_tokens: None,
    });

    // The gemini runner does not report a per-turn cost. We bill purely
    // off the cost_multiplier × token volume that downstream layers
    // compute; per-call cost is left as zero here. (BYOK is rejected
    // upstream, so the BYOK arm is inert, kept for parity with codex.)
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
fn base_chunk(
    id: String,
    created: u64,
    upstream: Upstream,
    agent_instance_hierarchy: String,
    agent_id: String,
    agent_full_id: String,
    agent_remote: Option<objectiveai_sdk::RemotePath>,
    messages: Vec<MessageChunk>,
) -> AgentCompletionChunk {
    AgentCompletionChunk {
        id,
        agent_instance_hierarchy,
        agent_id,
        agent_full_id,
        agent_remote,
        created,
        messages,
        object: Default::default(),
        usage: None,
        upstream,
        error: None,
        continuation: None,
        messages_queued: None,
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
    tool_calls: Option<Vec<AssistantToolCallDelta>>,
    finish_reason: Option<FinishReason>,
    usage: Option<UpstreamUsage>,
    agent_instance_hierarchy: String,
    agent_id: String,
    agent_full_id: String,
    agent_remote: Option<objectiveai_sdk::RemotePath>,
) -> AgentCompletionChunk {
    base_chunk(
        id,
        created,
        upstream,
        agent_instance_hierarchy,
        agent_id,
        agent_full_id,
        agent_remote,
        vec![MessageChunk::Assistant(AssistantResponseChunk {
            index: assistant_index,
            created,
            model,
            upstream_id,
            reasoning,
            tool_calls,
            content,
            finish_reason,
            usage,
            ..Default::default()
        })],
    )
}

/// Map one [`GeminiEvent`] to an optional downstream chunk.
///
/// `assistant_index` is the current assistant-message index (managed by
/// the streaming task). Returns `None` for events that produce no
/// downstream chunk.
#[allow(clippy::too_many_arguments)]
pub fn into_downstream(
    event: GeminiEvent,
    id: String,
    created: u64,
    model: String,
    assistant_index: u64,
    is_byok: bool,
    cost_multiplier: rust_decimal::Decimal,
    upstream: Upstream,
    agent_instance_hierarchy: String,
    agent_id: String,
    agent_full_id: String,
    agent_remote: Option<objectiveai_sdk::RemotePath>,
) -> Option<AgentCompletionChunk> {
    match event {
        GeminiEvent::Text { text } => {
            if text.is_empty() {
                return None;
            }
            Some(assistant_chunk(
                id,
                created,
                model,
                upstream,
                String::new(),
                assistant_index,
                Some(RichContent::Text(text)),
                None,
                None,
                None,
                None,
                agent_instance_hierarchy,
                agent_id,
                agent_full_id,
                agent_remote,
            ))
        }
        GeminiEvent::Thinking { text } => {
            if text.is_empty() {
                return None;
            }
            Some(assistant_chunk(
                id,
                created,
                model,
                upstream,
                String::new(),
                assistant_index,
                None,
                Some(text),
                None,
                None,
                None,
                agent_instance_hierarchy,
                agent_id,
                agent_full_id,
                agent_remote,
            ))
        }
        GeminiEvent::ToolUse { id: call_id, name, input } => {
            // The runner already dispatched this call internally; we
            // surface it as an assistant `tool_call` for visibility.
            let arguments = serde_json::to_string(&input).ok();
            let tool_call = AssistantToolCallDelta {
                index: 0,
                r#type: Some(AssistantToolCallType::Function),
                id: Some(call_id),
                function: Some(AssistantToolCallFunctionDelta {
                    name: Some(name),
                    arguments,
                }),
            };
            Some(assistant_chunk(
                id,
                created,
                model,
                upstream,
                String::new(),
                assistant_index,
                None,
                None,
                Some(vec![tool_call]),
                None,
                None,
                agent_instance_hierarchy,
                agent_id,
                agent_full_id,
                agent_remote,
            ))
        }
        GeminiEvent::ToolResult {
            tool_use_id,
            content,
            is_error: _,
        } => {
            // `is_error` is informational only; the SDK `ToolMessage`
            // has no error flag, so it's carried in the text body when
            // the runner prefixes it. Emit a `Tool` chunk so the
            // downstream consumer sees the result. This also "finishes"
            // the current assistant index for the caller.
            let tool_msg = ToolMessage {
                content: RichContent::Text(content),
                tool_call_id: tool_use_id,
                metadata: None,
            };
            Some(base_chunk(
                id,
                created,
                upstream,
                agent_instance_hierarchy,
                agent_id,
                agent_full_id,
                agent_remote,
                vec![MessageChunk::Tool(ToolResponse {
                    role: Default::default(),
                    index: assistant_index,
                    inner: tool_msg,
                    request_message_ids: None,
                })],
            ))
        }
        GeminiEvent::Usage {
            input_tokens,
            output_tokens,
            thinking_tokens: _,
            total_tokens: _,
        } => {
            let usage = upstream_usage(
                input_tokens,
                output_tokens,
                is_byok,
                cost_multiplier,
            );
            Some(assistant_chunk(
                id,
                created,
                model,
                upstream,
                String::new(),
                assistant_index,
                None,
                None,
                None,
                Some(FinishReason::Stop),
                Some(usage),
                agent_instance_hierarchy,
                agent_id,
                agent_full_id,
                agent_remote,
            ))
        }
        GeminiEvent::Unknown => None,
    }
}
