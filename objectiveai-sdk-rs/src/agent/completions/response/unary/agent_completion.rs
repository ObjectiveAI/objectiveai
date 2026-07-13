//! Agent completion response type.

use crate::agent::completions::response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A complete agent completion response.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema,
)]
#[schemars(rename = "agent.completions.response.unary.AgentCompletion")]
pub struct AgentCompletion {
    pub id: String,
    /// Full agent instance hierarchy for this completion's slot. See
    /// [`super::streaming::AgentCompletionChunk::agent_instance_hierarchy`].
    pub agent_instance_hierarchy: String,
    /// Leaf agent id of the slot that produced this completion. See
    /// [`super::streaming::AgentCompletionChunk::agent_id`].
    pub agent_id: String,
    /// WF-level id: see
    /// [`super::streaming::AgentCompletionChunk::agent_full_id`].
    pub agent_full_id: String,
    /// `RemotePath` the WF was fetched from, or `None` when inline.
    /// See [`super::streaming::AgentCompletionChunk::agent_remote`].
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_remote: Option<crate::RemotePath>,
    pub created: u64,
    pub messages: Vec<super::Message>,
    /// The object type (always "agent.completion").
    pub object: super::Object,
    pub usage: response::Usage,
    /// Upstream provider
    pub upstream: crate::agent::Upstream,
    /// Error details if this completion failed.
    pub error: Option<crate::error::ResponseError>,
    /// Continuation state for multi-turn conversations.
    pub continuation: Option<String>,
    /// `true` when the MCP proxy holds queued messages that were not
    /// delivered to the agent via a tool response on this turn. See
    /// [`super::streaming::AgentCompletionChunk::messages_queued`].
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub messages_queued: Option<bool>,
}

impl AgentCompletion {
    /// Normalize non-deterministic fields for test snapshot comparison.
    pub fn normalize_for_tests(&mut self) {
        self.id = String::new();
        self.agent_instance_hierarchy = String::new();
        self.created = 0;
        // Durations AND costs vary run-to-run (script agents record
        // real wall time, and the duration charge lands in the cost
        // figures); strip both from the aggregate AND from each
        // assistant turn's usage so duration-recording upstreams are
        // valid snapshot targets.
        self.usage.normalize_for_tests();
        for msg in &mut self.messages {
            if let super::Message::Assistant(asst) = msg {
                asst.upstream_id = String::new();
                asst.created = 0;
                asst.usage.normalize_for_tests();
            }
        }

        // The continuation is base64-encoded JSON whose payload includes
        // the agent's lineage `agent_instance_hierarchy` (minted at
        // first-spawn from a UUID + creation timestamp), which varies
        // run-to-run and would break every snapshot otherwise. Decode,
        // clear it, re-encode.
        if let Some(s) = &mut self.continuation {
            if let Some(mut c) = crate::agent::Continuation::try_from_string(s)
            {
                c.set_agent_instance_hierarchy(String::new());
                *s = c.to_string();
            }
        }
    }
}

impl From<response::streaming::AgentCompletionChunk> for AgentCompletion {
    fn from(
        response::streaming::AgentCompletionChunk {
            id,
            agent_instance_hierarchy,
            agent_id,
            agent_full_id,
            agent_remote,
            created,
            messages,
            object,
            usage,
            upstream,
            error,
            continuation,
            messages_queued,
        }: response::streaming::AgentCompletionChunk,
    ) -> Self {
        Self {
            id,
            agent_instance_hierarchy,
            agent_id,
            agent_full_id,
            agent_remote,
            created,
            messages: messages.into_iter().map(Into::into).collect(),
            object: object.into(),
            usage: usage.unwrap_or_default(),
            upstream,
            error,
            continuation,
            messages_queued,
        }
    }
}
