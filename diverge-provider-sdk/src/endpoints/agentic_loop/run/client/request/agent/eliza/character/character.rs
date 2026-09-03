//! The character.

use serde::{Deserialize, Serialize};

use super::{Example, Style};

/// Who the agent is, as Eliza renders it.
///
/// [`name`](Self::name) is the one required field — Eliza's own
/// minimum, and the prompt's `{{agentName}}`. Everything else is
/// an ingredient Eliza samples or blocks into the prompt, empty
/// meaning "nothing to say", which is how a caller whose whole
/// personality is the system prompt states that.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Character {
    /// The agent's name. The prompt's `{{agentName}}` and
    /// `{{name}}`; every prompt block is headed by it.
    pub name: String,
    /// The system prompt head. Absent gives Eliza's own: "You are
    /// {{name}}, an autonomous AI agent powered by elizaOS."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Biography lines. Blocked whole into the canonical system
    /// prompt as "About {{name}}", and sampled (up to ten,
    /// deterministically) by the character provider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bio: Vec<String>,
    /// Interests. One is rendered as what the agent is "currently
    /// interested in" and up to five more as "also interested in",
    /// picked deterministically per room.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// Traits. One is rendered as "{{name}} is {{adjective}}", picked
    /// deterministically per room.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adjectives: Vec<String>,
    /// Style directions. See [`Style`].
    #[serde(default, skip_serializing_if = "Style::is_empty")]
    pub style: Style,
    /// Example conversations, each a group of turns in order. Five
    /// groups are sampled into chat-room prompts, deterministically
    /// per room. See [`Example`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_examples: Vec<Vec<Example>>,
}
