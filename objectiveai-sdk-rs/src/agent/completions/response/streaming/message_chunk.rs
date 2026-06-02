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
#[serde(untagged)]
#[schemars(rename = "agent.completions.response.streaming.MessageChunk")]
pub enum MessageChunk {
    #[schemars(title = "Assistant")]
    Assistant(super::AssistantResponseChunk),
    #[schemars(title = "Tool")]
    Tool(super::super::ToolResponse),
}

impl MessageChunk {
    pub fn index(&self) -> u64 {
        match self {
            MessageChunk::Assistant(chunk) => chunk.index,
            MessageChunk::Tool(chunk) => chunk.index,
        }
    }

    pub fn push(&mut self, other: &MessageChunk) {
        match (self, other) {
            (
                MessageChunk::Assistant(self_chunk),
                MessageChunk::Assistant(other_chunk),
            ) => {
                self_chunk.push(other_chunk);
            }
            // _ => panic!("cannot push to or from tool chunks"),
            _ => {}
        }
    }

}
