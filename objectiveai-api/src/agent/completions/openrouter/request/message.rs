//! The wire-level message type sent in an OpenRouter request body.

use serde::{Deserialize, Serialize};

/// A message in an OpenRouter request body.
///
/// This is an untagged superset of a conversation
/// [`Message`](objectiveai_sdk::agent::completions::message::Message): it adds
/// the agent's `system_prompt` as a leading, role-tagged entry that the
/// conversation `Message` enum can no longer represent. `SystemPrompt` already
/// serializes to OpenRouter's `{"role": "system"|"developer", "content": ...}`
/// shape, and `Message` carries its own `role` tag, so the untagged enum
/// produces the correct wire form for either arm. The two arms are
/// unambiguous to deserialize as well: `Message` no longer has a system /
/// developer role, and `SystemPrompt` only accepts those two roles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestMessage {
    /// The agent's system prompt, rendered as the leading message.
    System(objectiveai_sdk::agent::openrouter::system_prompt::SystemPrompt),
    /// A conversation message (user / assistant / tool).
    Conversation(objectiveai_sdk::agent::completions::message::Message),
}
