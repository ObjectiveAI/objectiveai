//! Agent completion notify parameters.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for the `agent.completions.notify` endpoint. Pushes a
/// user message at a running agent completion identified by
/// `response_id`; the message surfaces to the model on its next
/// natural inspection point. The endpoint has no response body — any
/// 2xx status is the success signal.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.request.AgentCompletionNotifyParams")]
pub struct AgentCompletionNotifyParams {
    /// The `id` of the agent completion response this message is for —
    /// the same value the api echoes in the streaming/unary response's
    /// `id` field.
    pub response_id: String,
    /// The user message content to inject (text or multi-part rich
    /// content).
    pub content: crate::agent::completions::message::RichContent,
}
