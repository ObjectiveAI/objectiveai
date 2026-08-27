//! What the API billed.

use serde::{Deserialize, Serialize};

/// `BetaUsage`: token counts for one API message.
///
/// The cache counts are nullable in the SDK — a model or tier that
/// does not report caching sends `null`, not `0`, and the difference
/// is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens billed, not counting cache reads or writes.
    pub input_tokens: u64,
    /// Output tokens billed.
    pub output_tokens: u64,
    /// Tokens written to the prompt cache, if reported.
    pub cache_creation_input_tokens: Option<u64>,
    /// Tokens read from the prompt cache, if reported.
    pub cache_read_input_tokens: Option<u64>,
}

/// `BetaMessageDeltaUsage`: the one cumulative count a
/// [`message_delta`](super::StreamEvent::MessageDelta) carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeltaUsage {
    /// The cumulative number of output tokens so far.
    pub output_tokens: u64,
}
