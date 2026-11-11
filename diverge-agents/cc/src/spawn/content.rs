//! What this agent makes of a message's content blocks: Claude
//! Code's own.
//!
//! A message is MCP content — text, an image, audio, an embedded
//! resource, a link to one — and Claude Code's stream-json input
//! takes a user message as the Anthropic API's content blocks: text,
//! and an image by its bytes in one of four formats. A block this
//! cannot become refuses the WHOLE message, at `/run` before the
//! session starts and at `/enqueue` before the line is written, so
//! its fate is the error and nothing is silently reduced to its
//! text.

use rmcp::model::{ContentBlock, ResourceContents};
use serde_json::Value;

use super::stdin::{Block, ImageSource};

/// The refusal for a block this agent cannot take.
fn unsupported(kind: &str) -> Value {
    serde_json::json!({
        "kind": "content",
        "error": format!("{kind} content is not supported by this agent"),
    })
}

/// The four image formats the Anthropic API takes.
fn image_block(mime_type: &str, data: &str) -> Result<Block, Value> {
    match mime_type {
        "image/jpeg" | "image/png" | "image/gif" | "image/webp" => Ok(Block::Image {
            source: ImageSource::Base64 {
                media_type: mime_type.to_string(),
                data: data.to_string(),
            },
        }),
        other => Err(unsupported(&format!("{other} image"))),
    }
}

/// A message as Claude Code's blocks, or the refusal of the whole
/// message: text as text; an image as an image; an embedded text
/// resource as text and an embedded image as an image; a link as its
/// name and URI; audio, any other binary resource, and a block this
/// crate does not know refused. A message with no block is refused
/// too.
pub fn blocks(content: &[ContentBlock]) -> Result<Vec<Block>, Value> {
    if content.is_empty() {
        return Err(serde_json::json!({
            "kind": "content",
            "error": "a message needs content",
        }));
    }
    let mut blocks = Vec::with_capacity(content.len());
    for block in content {
        blocks.push(match block {
            ContentBlock::Text(text) => Block::Text { text: text.text.clone() },
            ContentBlock::Image(image) => image_block(&image.mime_type, &image.data)?,
            ContentBlock::Audio(_) => return Err(unsupported("audio")),
            ContentBlock::Resource(embedded) => match &embedded.resource {
                ResourceContents::TextResourceContents { text, .. } => Block::Text { text: text.clone() },
                ResourceContents::BlobResourceContents { blob, mime_type, .. } => {
                    image_block(mime_type.as_deref().unwrap_or(""), blob)?
                }
                _ => return Err(unsupported("unknown resource")),
            },
            ContentBlock::ResourceLink(resource) => Block::Text {
                text: format!("{} <{}>", resource.name, resource.uri),
            },
            _ => return Err(unsupported("unknown")),
        });
    }
    Ok(blocks)
}
