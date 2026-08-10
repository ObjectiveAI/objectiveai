//! The agentic loop request.

use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};

use super::Agent;

/// What a caller hands a provider to start or resume a loop.
///
/// One shape for both. A resume is this same request with
/// [`continuation`](Self::continuation) set — not a second request
/// type — because everything else still applies: the agent's
/// parameters can change between turns, and a resume that could not
/// express that would force a caller to start over to alter them.
///
/// **Everything here is post-transform.** An agent as authored can
/// carry a system prompt, prefix and suffix messages, a personality;
/// those shape a request before a provider sees it, and by the time
/// one of these is built they have already been applied.
/// [`prompt`](Self::prompt) is the result, not the ingredients, so a
/// provider never rewrites what it was given.
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
    /// The input for this turn.
    ///
    /// A prompt, not a conversation. What came before lives in
    /// [`continuation`](Self::continuation), which is the provider's
    /// own state — so a caller sends what is NEW and never
    /// reconstructs a history it would have to keep a parallel record
    /// of.
    ///
    /// Content blocks rather than a message, because the role is
    /// implied: a caller can only ever speak as itself.
    pub prompt: Vec<ContentBlock>,
    /// Resume a loop, using the token from its
    /// [`ContinuationChunk`](crate::agentic_loop::response::ContinuationChunk).
    ///
    /// Opaque: a caller stores it and hands it back, and should read
    /// nothing into its contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
}
