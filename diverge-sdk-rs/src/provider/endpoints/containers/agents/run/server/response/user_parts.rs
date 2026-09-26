//! A delivered message as its parts.

use rmcp::model::ContentBlock;

use super::{
    AgenticLoopChunk, UserAudioContentChunk, UserImageContentChunk, UserResourceChunk, UserResourceLinkChunk,
    UserTextContentChunk,
};

/// One chunk per block of a delivered message, in the message's
/// order, each under the message's `key`: what every image yields
/// at the position a message landed, and first of all for the
/// message a run started on.
///
/// A block of a kind this crate does not know — MCP's content is
/// open-ended — yields nothing: the image refused the message before
/// it was taken, so none reaches here.
pub fn user_parts(key: &str, content: Vec<ContentBlock>) -> Vec<AgenticLoopChunk> {
    content
        .into_iter()
        .filter_map(|block| match block {
            ContentBlock::Text(inner) => Some(AgenticLoopChunk::UserTextContent(UserTextContentChunk {
                r#type: Default::default(),
                key: key.to_string(),
                inner,
            })),
            ContentBlock::Image(inner) => Some(AgenticLoopChunk::UserImageContent(UserImageContentChunk {
                r#type: Default::default(),
                key: key.to_string(),
                inner,
            })),
            ContentBlock::Audio(inner) => Some(AgenticLoopChunk::UserAudioContent(UserAudioContentChunk {
                r#type: Default::default(),
                key: key.to_string(),
                inner,
            })),
            ContentBlock::Resource(inner) => Some(AgenticLoopChunk::UserResource(UserResourceChunk {
                r#type: Default::default(),
                key: key.to_string(),
                inner,
            })),
            ContentBlock::ResourceLink(inner) => Some(AgenticLoopChunk::UserResourceLink(UserResourceLinkChunk {
                r#type: Default::default(),
                key: key.to_string(),
                inner,
            })),
            _ => None,
        })
        .collect()
}
