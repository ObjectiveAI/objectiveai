//! The usage chunk.

use rmcp::model::MetaObject;
use serde::{Deserialize, Serialize};

/// Token usage.
///
/// Emitted as the loop goes rather than once at the end, so a caller
/// watches consumption grow instead of learning it after the fact.
/// Each chunk is a DELTA — every field is additive, so a caller that
/// wants a running total sums them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UsageChunk {
    /// The discriminator. See [`AgenticLoopChunk`](super::AgenticLoopChunk).
    pub r#type: UsageChunkType,
    /// Tokens generated.
    pub completion_tokens: u64,
    /// Prompt tokens consumed.
    pub prompt_tokens: u64,
    /// The two above, summed.
    pub total_tokens: u64,
    /// Arbitrary protocol-level metadata, MCP's `_meta` extension bag.
    ///
    /// Same key and same type as the chunks that flatten rmcp types
    /// carry, so a trace id attached to a content chunk can be
    /// attached here too — these three are ours rather than MCP's, but
    /// that is no reason for them to be the one place a trace stops.
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// [`UsageChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UsageChunkType {
    #[serde(rename = "usage")]
    #[default]
    Usage,
}
