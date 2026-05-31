//! Terminal notifications for `agents message`. Exactly one of these
//! lands on stdout per invocation:
//!
//! - [`MessageDelivered`] — the rich-content message was written into
//!   the target agent's live socket and acknowledged by the running
//!   cli-stream listener. The agent is awake and processing it.
//! - [`MessageQueued`] — the live socket was unreachable, so the cli
//!   resumed the agent's most-recent completion via continuation. A
//!   new agent-completion stream is now running in the background;
//!   `response_id` identifies that fresh response. The original
//!   `agent_id` is reused — continuations don't mint a new lineage.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Live-delivery success.
///
/// Wire: `{"type":"notification","value":{"kind":"message_delivered","agent_id":"<id>"}}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.agents.MessageDelivered")]
pub struct MessageDelivered {
    /// Full lineage agent_id the message was delivered to.
    pub agent_id: String,
}

/// Continuation-fallback success — the agent was dormant, so a new
/// completion stream was started from the most recent continuation.
///
/// Wire: `{"type":"notification","value":{"kind":"message_queued","agent_id":"<id>","response_id":"<rid>"}}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.agents.MessageQueued")]
pub struct MessageQueued {
    /// Full lineage agent_id (reused from the prior completion).
    pub agent_id: String,
    /// New response id for the continuation completion.
    pub response_id: String,
}
