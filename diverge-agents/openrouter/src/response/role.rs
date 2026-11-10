//! Role type for responses.

use serde::Deserialize;

/// The role of a message in a response (always "assistant").
#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub enum Role {
    /// The assistant role.
    #[serde(rename = "assistant")]
    #[default]
    Assistant,
}
