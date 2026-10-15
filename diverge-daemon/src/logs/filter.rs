//! A logs request's filter over one item.

use diverge_sdk::daemon::endpoints::agents::logs::client::request::{Frame, ItemType};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{Item, ItemWrapper};
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;

/// Whether the item is one the request asks for, by its type and the
/// time span. The index span is the reader's: it chooses which items
/// to look at, and this says which of those to send.
pub fn matches(frame: &Frame, wrapper: &ItemWrapper) -> bool {
    frame.r#type.is_none_or(|wanted| item_type(&wrapper.item) == wanted)
        && frame.created_from.is_none_or(|from| wrapper.created >= from)
        && frame.created_to.is_none_or(|to| wrapper.created <= to)
}

/// The kind of the item, as a request's `type` names one.
pub fn item_type(item: &Item) -> ItemType {
    match item {
        Item::User(user) => chunk_type(&user.chunk),
        Item::Chunk(chunk) => chunk_type(chunk),
        Item::Error(_) => ItemType::Error,
        Item::Active(_) => ItemType::Active,
        Item::Inactive(_) => ItemType::Inactive,
    }
}

/// The kind of a chunk.
fn chunk_type(chunk: &AgenticLoopChunk) -> ItemType {
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
