//! The kinds of item a log holds, by name, across every book.

use serde::{Deserialize, Serialize};

/// One kind of item, by the name its `type` carries: every name an
/// [`agent_log`](crate::daemon::agent_log) or a
/// [`tool_log`](crate::daemon::tool_log) item has. A name both books
/// have — `active`, `inactive`, `error`, `notification` — picks that
/// kind in whichever book is read; a name the book read does not have
/// matches nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// `user_text_content`.
    UserTextContent,
    /// `user_image_content`.
    UserImageContent,
    /// `user_audio_content`.
    UserAudioContent,
    /// `user_resource`.
    UserResource,
    /// `user_resource_link`.
    UserResourceLink,
    /// `assistant_reasoning`.
    AssistantReasoning,
    /// `assistant_text_content`.
    AssistantTextContent,
    /// `assistant_image_content`.
    AssistantImageContent,
    /// `assistant_audio_content`.
    AssistantAudioContent,
    /// `assistant_tool_call`.
    AssistantToolCall,
    /// `assistant_refusal`.
    AssistantRefusal,
    /// `tool_response`.
    ToolResponse,
    /// `usage`.
    Usage,
    /// `notification`.
    Notification,
    /// `error`.
    Error,
    /// `active`.
    Active,
    /// `inactive`.
    Inactive,
    /// `call`.
    Call,
    /// `call_result`.
    CallResult,
    /// `call_error`.
    CallError,
    /// `read`.
    Read,
    /// `read_result`.
    ReadResult,
    /// `read_error`.
    ReadError,
}

impl ItemType {
    /// The kind's name, as `type` carries it on the wire.
    pub fn name(self) -> &'static str {
        match self {
            ItemType::UserTextContent => "user_text_content",
            ItemType::UserImageContent => "user_image_content",
            ItemType::UserAudioContent => "user_audio_content",
            ItemType::UserResource => "user_resource",
            ItemType::UserResourceLink => "user_resource_link",
            ItemType::AssistantReasoning => "assistant_reasoning",
            ItemType::AssistantTextContent => "assistant_text_content",
            ItemType::AssistantImageContent => "assistant_image_content",
            ItemType::AssistantAudioContent => "assistant_audio_content",
            ItemType::AssistantToolCall => "assistant_tool_call",
            ItemType::AssistantRefusal => "assistant_refusal",
            ItemType::ToolResponse => "tool_response",
            ItemType::Usage => "usage",
            ItemType::Notification => "notification",
            ItemType::Error => "error",
            ItemType::Active => "active",
            ItemType::Inactive => "inactive",
            ItemType::Call => "call",
            ItemType::CallResult => "call_result",
            ItemType::CallError => "call_error",
            ItemType::Read => "read",
            ItemType::ReadResult => "read_result",
            ItemType::ReadError => "read_error",
        }
    }
}
