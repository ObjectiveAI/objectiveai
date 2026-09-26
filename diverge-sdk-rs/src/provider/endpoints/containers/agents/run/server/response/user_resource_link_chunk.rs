//! The user resource link chunk.

use rmcp::model::Resource;
use serde::{Deserialize, Serialize};

/// A resource link of a delivered message.
///
/// One block of the message, whole, at the position the message
/// landed: between the tool responses it was folded in behind, or
/// opening the next turn when the assistant had already finished, or
/// first of all, for the message a run started on. A message's parts
/// arrive consecutively, in the message's order, each under its
/// [`key`](Self::key); a message is its run of parts under one key.
/// The stream's order is the only statement of WHERE; a part is the
/// statement of THAT, and of WHICH.
///
/// The payload is MCP's own [`Resource`], flattened, so what the
/// caller said is expressed in the same vocabulary a tool would use
/// to return it and the model would use to say it — one content
/// model across the whole loop rather than one per direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserResourceLinkChunk {
    /// The discriminator. See [`AgenticLoopChunk`](super::AgenticLoopChunk).
    pub r#type: UserResourceLinkChunkType,
    /// The message's key, as its enqueue gave it: what tells one
    /// message's parts from another's when several are in flight, and
    /// what a dequeue names to withdraw the message.
    pub key: String,
    /// The block itself.
    #[serde(flatten)]
    pub inner: Resource,
}

/// [`UserResourceLinkChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserResourceLinkChunkType {
    #[serde(rename = "user_resource_link")]
    #[default]
    UserResourceLink,
}
