//! The Eliza agent.

use serde::{Deserialize, Serialize};

use super::{Character, Embedding, Memory, Provider, Toolsets, Upstream};

/// An agent running against Eliza (elizaOS).
///
/// Every field here is HONORABLE: it names something the harness
/// hands the runtime it constructs — a character field Eliza
/// renders into its prompts, a provider plugin's settings, a
/// constructor option, a plugin the image carries — and nothing
/// else made the cut. The one place this vocabulary bends the
/// protocol's post-transform rule is [`character`](Self::character),
/// and the bend is stated there.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Agent {
    /// The discriminator. Always `eliza`.
    pub upstream: Upstream,
    /// Who the agent is: the fields Eliza re-renders into its prompt
    /// EVERY turn. A parameter of the call, not a transform applied
    /// upstream — Eliza will not take a finished prompt, so a caller
    /// who authored a personality renders it into these fields. See
    /// [`Character`].
    pub character: Character,
    /// The inference source, credentials and all: one OpenAI-
    /// compatible endpoint that every model tier resolves to. See
    /// [`Provider`].
    pub provider: Provider,
    /// The embedding source, likewise — a second, independent
    /// endpoint, because the one that speaks may not embed. Absent
    /// = no vectors: memories are stored without them and searched
    /// by keyword. See [`Embedding`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Embedding>,
    /// Eliza's LOCAL capabilities, one switch each — the plugins
    /// the image carries. (The caller's MCP tools ride beside these
    /// regardless; they are not in this vocabulary.) See
    /// [`Toolsets`].
    pub toolsets: Toolsets,
    /// What a turn accumulates beyond the conversation itself —
    /// reflection, long-term memory, relationships, planning — each
    /// a cost per turn and a growth of the continuation. See
    /// [`Memory`].
    pub memory: Memory,
}
