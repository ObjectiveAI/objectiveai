//! The output vocabulary of a script agent's code.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One message in a script agent's OUTPUT — the conversation shape
/// minus the `user` role, since a script speaks only as the assistant
/// (or records tool results); it never puts words in the user's mouth.
/// The script's output deserializes as an array of these.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "role")]
#[schemars(rename = "agent.script.OutputMessage")]
pub enum OutputMessage {
    /// An assistant message.
    #[schemars(title = "Assistant")]
    #[serde(rename = "assistant")]
    Assistant(super::super::completions::message::AssistantMessage),
    /// A tool message containing the result of a tool call.
    #[schemars(title = "Tool")]
    #[serde(rename = "tool")]
    Tool(super::super::completions::message::ToolMessage),
}

impl From<OutputMessage> for super::super::completions::message::Message {
    fn from(message: OutputMessage) -> Self {
        match message {
            OutputMessage::Assistant(message) => {
                super::super::completions::message::Message::Assistant(message)
            }
            OutputMessage::Tool(message) => {
                super::super::completions::message::Message::Tool(message)
            }
        }
    }
}
