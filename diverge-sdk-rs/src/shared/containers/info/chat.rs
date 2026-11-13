//! What an agent that chats says of itself.

use serde::{Deserialize, Serialize};

use super::Modalities;

/// How an agent's program chats: one member, required, and no other.
///
/// Stated by an agent's image, in the frontmatter's `chat`. Nothing
/// reads it yet: a message is sent to an agent whatever this says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Chat {
    /// What a message to it may carry beside text.
    pub prompt_modalities: Modalities,
}
