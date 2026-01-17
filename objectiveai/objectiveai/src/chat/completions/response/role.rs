//! Role type for chat completion responses.

use serde::{Deserialize, Serialize};

/// The role of a message in a response (always "assistant").
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Role {
    /// The assistant role.
    #[serde(rename = "assistant")]
    #[default]
    Assistant,
}
