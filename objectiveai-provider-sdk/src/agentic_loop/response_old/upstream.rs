//! Which upstream produced a response.

use serde::{Deserialize, Serialize};

/// The upstream backing an agent.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum Upstream {
    /// Unrecognized — the default so an unknown value round-trips
    /// rather than failing to deserialize.
    #[default]
    Unknown,
    Openrouter,
    ClaudeAgentSdk,
    CodexSdk,
    Script,
}
