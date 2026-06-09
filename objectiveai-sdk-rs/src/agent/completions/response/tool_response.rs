use crate::agent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.ToolResponse")]
pub struct ToolResponse {
    pub role: ToolRole,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[serde(flatten)]
    pub inner: agent::completions::message::ToolMessage,
    /// Mirrors `AssistantResponseChunk.request_message_ids` —
    /// the consumed `message_queue_contents.id`s. Currently never
    /// populated (the API stamps the assistant chunk instead);
    /// the field exists so the wire shape is symmetric across
    /// the two `MessageChunk` variants. Both `None` and
    /// `Some(empty)` are skipped on serialize.
    #[serde(default, skip_serializing_if = "request_message_ids_is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub request_message_ids: Option<Vec<i64>>,
}

fn request_message_ids_is_empty(opt: &Option<Vec<i64>>) -> bool {
    opt.as_ref().map_or(true, |v| v.is_empty())
}


#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Default,
    PartialEq,
    Eq,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.ToolRole")]
pub enum ToolRole {
    #[serde(rename = "tool")]
    #[default]
    Tool,
}
