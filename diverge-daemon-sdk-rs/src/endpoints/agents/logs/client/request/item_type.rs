//! The kinds of item a log holds, by name.

use serde::{Deserialize, Serialize};

/// One kind of log item, as the `type` member of an
/// [`ItemWrapper`] names it: one variant per chunk the agent's run
/// can stream, and one for the errors. What a request's
/// [`type`](super::Frame::type) picks by.
///
/// | on the wire | item |
/// |---|---|
/// | `user_text_content` | [`UserTextContentChunk`] |
/// | `user_image_content` | [`UserImageContentChunk`] |
/// | `user_audio_content` | [`UserAudioContentChunk`] |
/// | `user_resource` | [`UserResourceChunk`] |
/// | `user_resource_link` | [`UserResourceLinkChunk`] |
/// | `assistant_reasoning` | [`AssistantReasoningChunk`] |
/// | `assistant_text_content` | [`AssistantTextContentChunk`] |
/// | `assistant_image_content` | [`AssistantImageContentChunk`] |
/// | `assistant_audio_content` | [`AssistantAudioContentChunk`] |
/// | `assistant_tool_call` | [`AssistantToolCallChunk`] |
/// | `assistant_refusal` | [`AssistantRefusalChunk`] |
/// | `tool_response` | [`ToolResponseChunk`] |
/// | `usage` | [`UsageChunk`] |
/// | `notification` | [`NotificationChunk`] |
/// | `error` | [`Error`] |
///
/// [`ItemWrapper`]: crate::endpoints::agents::logs::server::response::ItemWrapper
/// [`Error`]: crate::endpoints::agents::logs::server::response::Error
/// [`UserTextContentChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::UserTextContentChunk
/// [`UserImageContentChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::UserImageContentChunk
/// [`UserAudioContentChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::UserAudioContentChunk
/// [`UserResourceChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::UserResourceChunk
/// [`UserResourceLinkChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::UserResourceLinkChunk
/// [`AssistantReasoningChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::AssistantReasoningChunk
/// [`AssistantTextContentChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::AssistantTextContentChunk
/// [`AssistantImageContentChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::AssistantImageContentChunk
/// [`AssistantAudioContentChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::AssistantAudioContentChunk
/// [`AssistantToolCallChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::AssistantToolCallChunk
/// [`AssistantRefusalChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::AssistantRefusalChunk
/// [`ToolResponseChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::ToolResponseChunk
/// [`UsageChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::UsageChunk
/// [`NotificationChunk`]: diverge_provider_sdk::endpoints::containers::agents::run::server::response::NotificationChunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemType {
    /// One text block of a delivered message.
    #[serde(rename = "user_text_content")]
    UserTextContent,
    /// One image block of a delivered message.
    #[serde(rename = "user_image_content")]
    UserImageContent,
    /// One audio block of a delivered message.
    #[serde(rename = "user_audio_content")]
    UserAudioContent,
    /// One embedded resource of a delivered message.
    #[serde(rename = "user_resource")]
    UserResource,
    /// One resource link of a delivered message.
    #[serde(rename = "user_resource_link")]
    UserResourceLink,
    /// The agent's reasoning.
    #[serde(rename = "assistant_reasoning")]
    AssistantReasoning,
    /// Text the agent said.
    #[serde(rename = "assistant_text_content")]
    AssistantTextContent,
    /// An image the agent produced.
    #[serde(rename = "assistant_image_content")]
    AssistantImageContent,
    /// Audio the agent produced.
    #[serde(rename = "assistant_audio_content")]
    AssistantAudioContent,
    /// A tool call the agent made.
    #[serde(rename = "assistant_tool_call")]
    AssistantToolCall,
    /// A refusal.
    #[serde(rename = "assistant_refusal")]
    AssistantRefusal,
    /// A tool's answer to a call.
    #[serde(rename = "tool_response")]
    ToolResponse,
    /// Tokens consumed and produced.
    #[serde(rename = "usage")]
    Usage,
    /// A notification the run raised.
    #[serde(rename = "notification")]
    Notification,
    /// An error a run answered with.
    #[serde(rename = "error")]
    Error,
}
