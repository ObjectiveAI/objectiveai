//! The agentic loop request.

use rmcp::model::{MetaObject, Tool};
use serde::{Deserialize, Serialize};

use super::{Agent, Message};

/// What a caller hands a provider to start or resume a loop.
///
/// One shape for both. A resume is this same request with
/// [`continuation`](Self::continuation) set — not a second request
/// type — because everything else still applies: the tool set can
/// change between turns, the agent's parameters can change, and a
/// resume that could not express those would force a caller to start
/// over to alter either.
///
/// **Everything here is post-transform.** An agent as authored can
/// carry a system prompt, prefix and suffix messages, a personality;
/// those shape a request before a provider sees it, and by the time
/// one of these is built they have already been applied.
/// [`messages`](Self::messages) is the result, not the ingredients, so
/// a provider never rewrites a conversation — it sends what it was
/// given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgenticLoopRequest {
    /// What to run, and how to sample it.
    ///
    /// The model and every decoding parameter live here rather than on
    /// the request, because which parameters exist DEPENDS on the
    /// upstream — `logit_bias` is meaningless to the Claude Agent SDK,
    /// `thinking` is meaningless to OpenRouter, and a Python agent
    /// samples nothing at all.
    pub agent: Agent,
    /// The conversation, oldest first.
    ///
    /// On a resume this is the conversation as the caller now holds
    /// it, not only what is new. The provider is free to trust its own
    /// state instead; what it must not do is require the caller to
    /// have kept a separate record of what was already sent.
    pub messages: Vec<Message>,
    /// The tools the model may call.
    ///
    /// Already resolved — this is the tool list, not the MCP servers
    /// to go and ask. MCP's [`Tool`], so a list obtained from a server
    /// passes straight through without re-encoding a JSON Schema that
    /// was already one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
    /// Resume a loop, using the token from its
    /// [`ContinuationChunk`](crate::agentic_loop::response::ContinuationChunk).
    ///
    /// Opaque: a caller stores it and hands it back, and should read
    /// nothing into its contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    /// How many model turns the loop may take before stopping.
    ///
    /// The one bound a single completion never needed. An agentic loop
    /// calls tools and calls the model again with the results, which
    /// terminates only when the model decides to stop — so without a
    /// ceiling, a model that keeps calling tools runs until something
    /// external kills it. `None` leaves the ceiling to the provider,
    /// which must have one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    /// Seed for the sampler, where the upstream supports one.
    ///
    /// On the request rather than the agent because it is a property
    /// of this run, not of the agent — the same agent seeded
    /// differently is the same agent. A hint, never a guarantee: no
    /// provider promises a seed reproduces an output across model or
    /// backend changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Arbitrary protocol-level metadata, MCP's `_meta` extension bag.
    ///
    /// The same bag every response chunk carries — a `traceparent` set
    /// here is where a trace through the loop begins.
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}
