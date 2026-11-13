//! What an agent that chats says of itself.

use serde::{Deserialize, Serialize};

use super::Modalities;

/// How an agent's program chats: one member, required, and no other.
///
/// Stated by an agent's image, in the frontmatter's `chat`. The
/// daemon holds a message to it: one for an agent whose image states
/// `false` is refused, and so is one carrying what
/// [`prompt_modalities`](Self::prompt_modalities) does not take, see
/// [`Modalities::lacking`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Chat {
    /// What a message to it may carry beside text.
    pub prompt_modalities: Modalities,
}
