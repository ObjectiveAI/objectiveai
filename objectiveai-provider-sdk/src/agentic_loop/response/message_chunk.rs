//! One message in the loop — an assistant turn or a tool result.

use serde::{Deserialize, Serialize};

use super::{AssistantResponseChunk, ToolResponse};

/// One message of the conversation the loop is building.
///
/// Untagged, discriminated by the `role` each variant carries — so the
/// wire shape is a message, not a wrapper around one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageChunk {
    /// A model turn, possibly still streaming.
    Assistant(AssistantResponseChunk),
    /// A tool result, always complete.
    Tool(ToolResponse),
}

impl MessageChunk {
    /// This message's position in the loop. Assistant turns and tool
    /// results share one sequence, so this is unique across both.
    pub fn index(&self) -> u64 {
        match self {
            MessageChunk::Assistant(chunk) => chunk.index,
            MessageChunk::Tool(chunk) => chunk.index,
        }
    }

    /// Accumulate another chunk with the same index.
    ///
    /// Only assistant turns accumulate — a tool result arrives whole,
    /// so a second one for the same index is not a delta and is
    /// ignored rather than merged. Mismatched variants are likewise
    /// ignored: dropping a nonsensical pairing is preferable to
    /// panicking inside a fold that runs on untrusted input.
    pub fn push(&mut self, other: &MessageChunk) {
        if let (MessageChunk::Assistant(this), MessageChunk::Assistant(that)) =
            (self, other)
        {
            this.push(that);
        }
    }
}
