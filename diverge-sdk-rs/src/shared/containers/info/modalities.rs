//! Which kinds of content, beside text, an agent takes.

use serde::{Deserialize, Serialize};

/// The kinds of content an agent's program takes beside text, each
/// said outright: three members, all required, and no other.
///
/// Text is not among them: every agent takes text, and there is
/// nothing to say. A resource link is not among them either: it is a
/// name and an address, which an agent is handed as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Modalities {
    /// Whether it takes an image: an MCP image block.
    pub image: bool,
    /// Whether it takes audio: an MCP audio block.
    pub audio: bool,
    /// Whether it takes a file: an MCP embedded resource whose
    /// contents are bytes and are neither an image nor audio.
    pub file: bool,
}
