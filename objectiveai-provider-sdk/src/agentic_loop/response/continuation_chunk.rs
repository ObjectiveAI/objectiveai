//! The continuation chunk.

use serde::{Deserialize, Serialize};

/// The loop's resume token.
///
/// Pass `continuation` back to pick the conversation up where it
/// stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationChunk {
    /// The discriminator. Fixed, and the reason
    /// [`AgenticLoopChunk`](super::AgenticLoopChunk) can be untagged:
    /// serde has no tag of its own to read, so each variant's payload
    /// carries a `type` no other variant can match.
    pub r#type: ContinuationChunkType,
    /// Opaque state. Meaningful only to the provider that issued it —
    /// a consumer stores it and hands it back, and should read nothing
    /// into its contents.
    pub continuation: String,
}

/// [`ContinuationChunk`]'s discriminator.
///
/// One variant, so the field can hold exactly one value. A type rather
/// than a bare `String` because a wrong value then fails to
/// deserialize instead of arriving as data nobody checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ContinuationChunkType {
    #[serde(rename = "continuation")]
    #[default]
    Continuation,
}
