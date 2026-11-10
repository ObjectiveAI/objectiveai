//! One turn of an example conversation.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One turn in a group of [`message_examples`](super::Character::message_examples).
///
/// Eliza's `MessageExample` reduced to what renders: who spoke and
/// what they said. A speaker equal to the character's
/// [`name`](super::Character::name) is the agent's own turn; any
/// other name is a user's.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Example {
    /// Who spoke.
    pub name: String,
    /// What they said.
    pub text: String,
}
