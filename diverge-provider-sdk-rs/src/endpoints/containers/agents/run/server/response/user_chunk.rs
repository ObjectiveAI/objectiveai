//! The user chunk.

use rmcp::model::{ContentBlock, MetaObject};
use serde::{Deserialize, Serialize};

/// An enqueued message entering the conversation.
///
/// Emitted at the position the message landed: between the tool
/// responses it was folded in behind, or opening the next turn when
/// the assistant had already finished. The stream's order is the
/// only statement of WHERE; this chunk is the statement of THAT, and
/// of WHICH.
///
/// # It carries the content itself
///
/// The delivered message's content blocks, verbatim — so the chunk
/// stands on its own in the response stream and in any history built
/// from it, and a caller with several enqueues in flight tells them
/// apart by content. The
/// [`Delivered`](crate::shared::containers::enqueue::response::Frame::Delivered)
/// answer on the enqueue's own channel says the same event from the
/// channel's side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserChunk {
    /// The discriminator. Fixed, and the reason
    /// [`AgenticLoopChunk`](super::AgenticLoopChunk) can be untagged:
    /// serde has no tag of its own to read, so each variant's payload
    /// carries a `type` no other variant can match.
    pub r#type: UserChunkType,
    /// The delivered message's content, exactly as enqueued.
    pub content: Vec<ContentBlock>,
    /// Arbitrary protocol-level metadata, MCP's `_meta` extension bag.
    ///
    /// Same key and same type as the chunks that flatten rmcp types
    /// carry, so a trace id attached to a content chunk can be
    /// attached here too.
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// [`UserChunk`]'s discriminator.
///
/// One variant, so the field can hold exactly one value. A type rather
/// than a bare `String` because a wrong value then fails to
/// deserialize instead of arriving as data nobody checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserChunkType {
    #[serde(rename = "user")]
    #[default]
    User,
}
