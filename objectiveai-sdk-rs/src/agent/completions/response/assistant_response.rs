//! Role type for agent completion responses.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The role of a message in a response (always "assistant").
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.AssistantRole")]
pub enum AssistantRole {
    /// The assistant role.
    #[serde(rename = "assistant")]
    #[default]
    Assistant,
}
