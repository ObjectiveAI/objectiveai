//! Context compression.

use serde::{Deserialize, Serialize};

/// Which compression engine to enable when a request would otherwise
/// exceed the model's context window. Maps onto OpenRouter's
/// request-body `plugins` array.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextCompression {
    /// Drops content from the middle of the conversation. The only
    /// engine documented today.
    #[default]
    MiddleOut,
}
