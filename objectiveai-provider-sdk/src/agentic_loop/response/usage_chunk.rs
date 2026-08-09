//! The usage chunk.

use serde::{Deserialize, Serialize};

/// Token usage.
///
/// Emitted as the loop goes rather than once at the end, so a caller
/// watches consumption grow instead of learning it after the fact.
/// Each chunk is a DELTA — every field is additive, so a caller that
/// wants a running total sums them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UsageChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: UsageChunkType,
    /// Tokens generated.
    pub completion_tokens: u64,
    /// Prompt tokens consumed.
    pub prompt_tokens: u64,
    /// The two above, summed.
    pub total_tokens: u64,
}

/// [`UsageChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UsageChunkType {
    #[serde(rename = "usage")]
    #[default]
    Usage,
}
