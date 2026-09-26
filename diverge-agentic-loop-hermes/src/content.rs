//! What this agent makes of a message's content blocks.
//!
//! A message is MCP content — text, an image, audio, an embedded
//! resource, a link to one — and this harness takes text alone, because Hermes's `/v1/runs` takes its input as a string and nothing richer survives that field. A block
//! it cannot take refuses the WHOLE message, at `/run` before the
//! loop starts and at `/enqueue` before the message is queued, so
//! its fate is the error and nothing is silently reduced to its
//! text. [`check`] is that judgment; [`render`] is the text a checked message becomes, its blocks joined by blank lines.

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
/// or one with an image, audio, a binary resource, or a block this
/// crate does not know.
pub fn check(content: &[ContentBlock]) -> Result<(), Value> {
    if content.is_empty() {
        return Err(empty());
    }
    for block in content {
        match block {
            ContentBlock::Text(_) | ContentBlock::ResourceLink(_) => {}
            ContentBlock::Image(_) => return Err(unsupported("image")),
            ContentBlock::Audio(_) => return Err(unsupported("audio")),
            ContentBlock::Resource(embedded) => match &embedded.resource {
                ResourceContents::TextResourceContents { .. } => {}
                ResourceContents::BlobResourceContents { .. } => return Err(unsupported("binary resource")),
                _ => return Err(unsupported("unknown resource")),
            },
            _ => return Err(unsupported("unknown")),
        }
    }
    Ok(())
}

/// A checked message as Hermes's input: every block's text, joined by
/// blank lines. A block [`check`] would have refused renders as
/// nothing; a message that reaches here was checked.
pub fn render(content: &[ContentBlock]) -> String {
    let mut texts = Vec::with_capacity(content.len());
    for block in content {
        match block {
            ContentBlock::Text(text) => texts.push(text.text.clone()),
            ContentBlock::ResourceLink(resource) => texts.push(link(&resource.name, &resource.uri)),
            ContentBlock::Resource(embedded) => {
                if let ResourceContents::TextResourceContents { text, .. } = &embedded.resource {
                    texts.push(text.clone());
                }
            }
            _ => {}
        }
    }
    texts.join("\n\n")
}
