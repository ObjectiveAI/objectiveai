//! The kinds of item a log holds, by name.

use serde::{Deserialize, Serialize};

use crate::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;

use super::Item;

/// One kind of log item, as the `type` member of an
/// [`ItemWrapper`] names it: one variant per chunk the agent's run
/// can stream, one for the errors, and two for the agent starting and
/// ceasing to run on a provider. What a request's
/// `type` picks by.
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
/// | `active` | [`Active`] |
/// | `inactive` | [`Inactive`] |
///
/// [`ItemWrapper`]: crate::daemon::agent_log::ItemWrapper
/// [`Error`]: crate::daemon::agent_log::Error
/// [`Active`]: crate::daemon::agent_log::Active
/// [`Inactive`]: crate::daemon::agent_log::Inactive
/// [`UserTextContentChunk`]: crate::provider::endpoints::containers::agents::run::server::response::UserTextContentChunk
/// [`UserImageContentChunk`]: crate::provider::endpoints::containers::agents::run::server::response::UserImageContentChunk
/// [`UserAudioContentChunk`]: crate::provider::endpoints::containers::agents::run::server::response::UserAudioContentChunk
/// [`UserResourceChunk`]: crate::provider::endpoints::containers::agents::run::server::response::UserResourceChunk
/// [`UserResourceLinkChunk`]: crate::provider::endpoints::containers::agents::run::server::response::UserResourceLinkChunk
/// [`AssistantReasoningChunk`]: crate::provider::endpoints::containers::agents::run::server::response::AssistantReasoningChunk
/// [`AssistantTextContentChunk`]: crate::provider::endpoints::containers::agents::run::server::response::AssistantTextContentChunk
/// [`AssistantImageContentChunk`]: crate::provider::endpoints::containers::agents::run::server::response::AssistantImageContentChunk
/// [`AssistantAudioContentChunk`]: crate::provider::endpoints::containers::agents::run::server::response::AssistantAudioContentChunk
/// [`AssistantToolCallChunk`]: crate::provider::endpoints::containers::agents::run::server::response::AssistantToolCallChunk
/// [`AssistantRefusalChunk`]: crate::provider::endpoints::containers::agents::run::server::response::AssistantRefusalChunk
/// [`ToolResponseChunk`]: crate::provider::endpoints::containers::agents::run::server::response::ToolResponseChunk
/// [`UsageChunk`]: crate::provider::endpoints::containers::agents::run::server::response::UsageChunk
/// [`NotificationChunk`]: crate::provider::endpoints::containers::agents::run::server::response::NotificationChunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
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
    /// The agent began running on a provider.
    #[serde(rename = "active")]
    Active,
    /// The agent ceased running on a provider.
    #[serde(rename = "inactive")]
    Inactive,
}

impl ItemType {
    /// The kind of `item`.
    pub fn of(item: &Item) -> ItemType {
        match item {
            Item::User(user) => ItemType::of_chunk(&user.chunk),
            Item::Chunk(chunk) => ItemType::of_chunk(chunk),
            Item::Error(_) => ItemType::Error,
            Item::Active(_) => ItemType::Active,
            Item::Inactive(_) => ItemType::Inactive,
        }
    }

    /// The kind of a chunk.
    fn of_chunk(chunk: &AgenticLoopChunk) -> ItemType {
        match chunk {
            AgenticLoopChunk::AssistantReasoning(_) => ItemType::AssistantReasoning,
            AgenticLoopChunk::AssistantTextContent(_) => ItemType::AssistantTextContent,
            AgenticLoopChunk::AssistantImageContent(_) => ItemType::AssistantImageContent,
            AgenticLoopChunk::AssistantAudioContent(_) => ItemType::AssistantAudioContent,
            AgenticLoopChunk::AssistantToolCall(_) => ItemType::AssistantToolCall,
            AgenticLoopChunk::AssistantRefusal(_) => ItemType::AssistantRefusal,
            AgenticLoopChunk::ToolResponse(_) => ItemType::ToolResponse,
            AgenticLoopChunk::UserTextContent(_) => ItemType::UserTextContent,
            AgenticLoopChunk::UserImageContent(_) => ItemType::UserImageContent,
            AgenticLoopChunk::UserAudioContent(_) => ItemType::UserAudioContent,
            AgenticLoopChunk::UserResource(_) => ItemType::UserResource,
            AgenticLoopChunk::UserResourceLink(_) => ItemType::UserResourceLink,
            AgenticLoopChunk::Usage(_) => ItemType::Usage,
            AgenticLoopChunk::Notification(_) => ItemType::Notification,
        }
    }

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
        }
    }
}
