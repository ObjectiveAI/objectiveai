//! The `agent_message` item: what the agent said.

use serde::Deserialize;

/// The agent's response: natural language, or a JSON string when
/// structured output was requested. Never started — it arrives whole
/// at `item.completed`, and the last one of a turn is the turn's
/// final message.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentMessage {
    /// The text, whole.
    pub text: String,
}
