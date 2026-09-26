//! The user resource chunk.

use rmcp::model::EmbeddedResource;
use serde::{Deserialize, Serialize};

/// An embedded resource of a delivered message.
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
/// The payload is MCP's own [`EmbeddedResource`], flattened, so what the
/// caller said is expressed in the same vocabulary a tool would use
/// to return it and the model would use to say it — one content
/// model across the whole loop rather than one per direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserResourceChunk {
    /// The discriminator. See [`AgenticLoopChunk`](super::AgenticLoopChunk).
    pub r#type: UserResourceChunkType,
    /// The message's key, as its enqueue gave it: what tells one
    /// message's parts from another's when several are in flight, and
    /// what a dequeue names to withdraw the message.
    pub key: String,
    /// The block itself.
    #[serde(flatten)]
    pub inner: EmbeddedResource,
}

/// [`UserResourceChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserResourceChunkType {
    #[serde(rename = "user_resource")]
    #[default]
    UserResource,
}
