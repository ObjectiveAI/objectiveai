//! Message type for unary agent completion responses.

use crate::agent::completions::response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.completions.response.unary.Message")]
pub enum Message {
    #[schemars(title = "Assistant")]
    Assistant(super::AssistantResponse),
    #[schemars(title = "Tool")]
    Tool(response::ToolResponse),
}

impl From<response::streaming::MessageChunk> for Message {
    fn from(chunk: response::streaming::MessageChunk) -> Self {
        match chunk {
            response::streaming::MessageChunk::Assistant(chunk) => {
                Message::Assistant(chunk.into())
            }
            response::streaming::MessageChunk::Tool(chunk) => {
                Message::Tool(chunk)
            }
        }
    }
}
