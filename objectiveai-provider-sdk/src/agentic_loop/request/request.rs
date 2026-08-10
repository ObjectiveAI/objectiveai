//! The agentic loop request.

use rmcp::model::{MetaObject, Tool};
use serde::{Deserialize, Serialize};

use super::Message;

/// What a caller hands a provider to start or resume a loop.
///
/// One shape for both. A resume is this same request with
/// [`continuation`](Self::continuation) set — not a second request
/// type — because everything else still applies: the tool set can
/// change between turns, the sampling parameters can change, and a
/// resume that could not express those would force a caller to start
/// over to alter either.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgenticLoopRequest {
    /// The model to run.
    ///
    /// A provider fronts many models, so this is not implied by which
    /// provider was asked.
    pub model: String,
    /// The conversation, oldest first.
    ///
    /// On a resume this is the conversation as the caller now holds
    /// it, not only what is new. The provider is free to trust its own
    /// state instead; what it must not do is require the caller to
    /// have kept a separate record of what was already sent.
    pub messages: Vec<Message>,
    /// The tools the model may call.
    ///
    /// MCP's [`Tool`], so a tool list obtained from an MCP server
    /// passes straight through — no re-encoding of a JSON Schema that
    /// was already a JSON Schema.
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
    /// Cap on tokens generated per turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Sampling temperature.
    ///
    /// A decimal rather than a float, for the reason
    /// [`Logprob::logprob`](crate::agentic_loop::response::Logprob::logprob)
    /// is: these round-trip through JSON, and a temperature that comes
    /// back as `0.7000000000000001` is a different request than the
    /// one that was sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<rust_decimal::Decimal>,
    /// Nucleus sampling threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<rust_decimal::Decimal>,
    /// Sequences that end a turn when generated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    /// Seed for the sampler, where the provider supports one.
    ///
    /// A hint, never a guarantee — no provider promises that the same
    /// seed reproduces the same output across model or backend
    /// changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Ask for log probabilities, with this many alternatives per
    /// token.
    ///
    /// `None` means do not report them; `Some(0)` means report the
    /// chosen token's probability and no alternatives. One field
    /// rather than a bool plus a count, so "logprobs off but
    /// alternatives requested" cannot be expressed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u8>,
    /// Arbitrary protocol-level metadata, MCP's `_meta` extension bag.
    ///
    /// The same bag every response chunk carries — a `traceparent` set
    /// here is where a trace through the loop begins.
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}
