//! API-local runner-wire conversation message shapes for the Gemini
//! runner.
//!
//! The Gemini runner is STATELESS: it holds no server-side session and
//! receives the entire conversation on every `run` request. The public
//! continuation persists the conversation in the canonical
//! [`completions::message::Message`] shape; at request time the API
//! translates that history into these runner-wire types (see
//! [`super::prompt`]), which serialize straight onto the `run` request
//! as the runner's `main.py` expects.
//!
//! These types are API-LOCAL (mirroring codex's API-local
//! `RunnerUserMessage` / `RunnerContentPart`). They are NOT exposed in
//! the SDK and never appear in a public continuation.
//!
//! [`completions::message::Message`]:
//!     objectiveai_sdk::agent::completions::message::Message

use serde::{Deserialize, Serialize};

/// One conversation message in the runner's wire shape, discriminated
/// by `role`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Message {
    /// A system instruction. Folded into the runner's
    /// `system_instruction` together with any agent `system_prompt`.
    System { content: Vec<ContentPart> },
    /// A user message (multimodal text/image parts).
    User { content: Vec<ContentPart> },
    /// A model (assistant) turn: text parts plus any tool calls the
    /// model emitted in that turn.
    Model {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<ContentPart>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
    },
    /// The result of a model tool call.
    Tool {
        tool_call_id: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        name: String,
        content: String,
        #[serde(default, skip_serializing_if = "is_false")]
        is_error: bool,
    },
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// One element of a message's `content`. The runner accepts `text`
/// parts and `image` parts (the latter referenced by URL).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// A text part: `{"type":"text","text":..}`.
    Text { text: String },
    /// An image part: `{"type":"image","url":..}` (data: or http(s):).
    Image {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
}

/// A tool call the model emitted in a `model` turn:
/// `{"id":..,"name":..,"args":{..}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}
