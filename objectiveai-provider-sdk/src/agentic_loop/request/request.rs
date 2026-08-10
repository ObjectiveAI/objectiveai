//! The agentic loop request.

use serde::{Deserialize, Serialize};

use super::{Agent, Message};

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
    /// Resume a loop, using the token from its
    /// [`ContinuationChunk`](crate::agentic_loop::response::ContinuationChunk).
    ///
    /// Opaque: a caller stores it and hands it back, and should read
    /// nothing into its contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
}
