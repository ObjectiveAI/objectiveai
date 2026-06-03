//! Object type for unary agent completion responses.

use crate::agent::completions::response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The object type for agent completion responses.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
)]
#[schemars(rename = "agent.completions.response.unary.Object")]
pub enum Object {
    /// A agent completion object.
    #[serde(rename = "agent.completion")]
    #[default]
    AgentCompletion,
}

impl From<response::streaming::Object> for Object {
    fn from(value: response::streaming::Object) -> Self {
        match value {
            response::streaming::Object::AgentCompletionChunk => {
                Object::AgentCompletion
            }
        }
    }
}
