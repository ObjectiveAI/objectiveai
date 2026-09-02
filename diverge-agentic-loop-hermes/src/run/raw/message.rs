//! One entry of the conversation history the run is handed.

use serde::Serialize;

/// One prior message, as `/v1/runs` takes it: a role and its text.
/// Nothing richer survives this field — the gateway reads `role`
/// and `content` as strings and nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Message {
    /// `user`, `assistant`, `system`, …, as the gateway spells them.
    pub role: String,
    /// The message text.
    pub content: String,
}
