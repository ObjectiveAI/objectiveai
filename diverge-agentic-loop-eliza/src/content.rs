//! What this agent makes of a message's content blocks.
//!
//! A message is MCP content — text, an image, audio, an embedded
//! resource, a link to one — and this harness takes every kind MCP defines: text as it is, an image described and audio transcribed through the runtime's own model tiers, a resource by its type, a link as its name and URI — the entry renders them, the way it renders a tool result. A block
//! it cannot take refuses the WHOLE message, at `/run` before the
//! loop starts and at `/enqueue` before the message is queued, so
//! its fate is the error and nothing is silently reduced to its
//! text. [`check`] is that judgment; the entry does the rendering, with the blocks handed to it whole.

use rmcp::model::{ContentBlock, ResourceContents};
use serde_json::Value;

/// The refusal for a block this agent cannot take.
fn unsupported(kind: &str) -> Value {
    serde_json::json!({
        "kind": "content",
        "error": format!("{kind} content is not supported by this agent"),
    })
}

/// The refusal for a message with nothing in it.
fn empty() -> Value {
    serde_json::json!({
        "kind": "content",
        "error": "a message needs content",
    })
}

/// A resource link, as the one line of text it can be.
fn link(name: &str, uri: &str) -> String {
    format!("{name} <{uri}>")
}

/// Refuse a message this agent cannot take whole: one with no block,
/// or one with a block this crate does not know. Every kind MCP
/// defines is taken; the entry renders it.
pub fn check(content: &[ContentBlock]) -> Result<(), Value> {
    if content.is_empty() {
        return Err(empty());
    }
    for block in content {
        match block {
            ContentBlock::Text(_)
            | ContentBlock::Image(_)
            | ContentBlock::Audio(_)
            | ContentBlock::ResourceLink(_) => {}
            ContentBlock::Resource(embedded) => match &embedded.resource {
                ResourceContents::TextResourceContents { .. } | ResourceContents::BlobResourceContents { .. } => {}
                _ => return Err(unsupported("unknown resource")),
            },
            _ => return Err(unsupported("unknown")),
        }
    }
    Ok(())
}

/// Kept for the module's shape: the entry renders, so there is no
/// text to make here. A resource link is the one block rendered on
/// this side, as its name and URI, so the entry sees a text block.
pub fn linked(content: Vec<ContentBlock>) -> Vec<ContentBlock> {
    content
        .into_iter()
        .map(|block| match block {
            ContentBlock::ResourceLink(resource) => ContentBlock::text(link(&resource.name, &resource.uri)),
            block => block,
        })
        .collect()
}
