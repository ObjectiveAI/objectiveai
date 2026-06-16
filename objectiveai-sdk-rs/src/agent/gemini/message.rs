//! Conversation message shapes carried in a Gemini [`Continuation`].
//!
//! The Gemini runner is STATELESS: it holds no server-side session and
//! receives the entire conversation on every `run` request. To resume a
//! conversation across separate `agent completions create` calls, the
//! API persists the full history in the continuation as a list of these
//! messages and replays them (prior history + this turn's messages) on
//! the next call.
//!
//! These types mirror the runner's accepted `messages` item schema
//! (`role` + per-role fields) 1:1 so they serialize straight onto the
//! `run` request without a translation layer.
//!
//! [`Continuation`]: super::Continuation

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One conversation message in the runner's wire shape, discriminated
/// by `role`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "role", rename_all = "snake_case")]
#[schemars(rename = "agent.gemini.Message")]
pub enum Message {
    /// A system instruction. Folded into the runner's
    /// `system_instruction` together with any agent `system_prompt`.
    #[schemars(title = "System")]
    System { content: Vec<ContentPart> },
    /// A user message (multimodal text/image parts).
    #[schemars(title = "User")]
    User { content: Vec<ContentPart> },
    /// A model (assistant) turn: text parts plus any tool calls the
    /// model emitted in that turn.
    #[schemars(title = "Model")]
    Model {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[schemars(extend("omitempty" = true))]
        content: Vec<ContentPart>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[schemars(extend("omitempty" = true))]
        tool_calls: Vec<ToolCall>,
    },
    /// The result of a model tool call.
    #[schemars(title = "Tool")]
    Tool {
        tool_call_id: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        #[schemars(extend("omitempty" = true))]
        name: String,
        content: String,
        #[serde(default, skip_serializing_if = "is_false")]
        #[schemars(extend("omitempty" = true))]
        is_error: bool,
    },
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// One element of a message's `content`. The runner accepts `text`
/// parts and `image` parts (the latter referenced by URL).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.gemini.ContentPart")]
pub enum ContentPart {
    /// A text part: `{"type":"text","text":..}`.
    #[schemars(title = "Text")]
    Text { text: String },
    /// An image part: `{"type":"image","url":..}` (data: or http(s):).
    #[schemars(title = "Image")]
    Image {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        mime_type: Option<String>,
    },
}

/// A tool call the model emitted in a `model` turn:
/// `{"id":..,"name":..,"args":{..}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.gemini.ToolCall")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}
