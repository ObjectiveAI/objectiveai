//! MCP content block enum.
//!
//! A content block is the union of all content types that can appear in
//! prompts, tool results, and sampling messages.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A content block that can be used in prompts and tool results.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
#[schemars(rename = "mcp.tool.ContentBlock")]
pub enum ContentBlock {
    /// Text content.
    #[serde(rename = "text")]
    #[schemars(title = "Text")]
    Text(super::TextContent),
    /// Image content (base64-encoded).
    #[serde(rename = "image")]
    #[schemars(title = "Image")]
    Image(super::ImageContent),
    /// Audio content (base64-encoded).
    #[serde(rename = "audio")]
    #[schemars(title = "Audio")]
    Audio(super::AudioContent),
    /// A resource link.
    #[serde(rename = "resource_link")]
    #[schemars(title = "ResourceLink")]
    ResourceLink(super::ResourceLink),
    /// An embedded resource.
    #[serde(rename = "resource")]
    #[schemars(title = "EmbeddedResource")]
    EmbeddedResource(super::EmbeddedResource),
}

/// Convert a single `RichContentPart` into a `ContentBlock`. Lossless
/// for text / data-URL image / audio. Non-data-URL image URLs and
/// video / file parts fall back to a `ContentBlock::Text` carrying
/// either the URL (image) or a JSON-serialized representation of the
/// part. Mirrors the agent-side [`From<ContentBlock> for RichContentPart`]
/// so that `RichContent` ↔ `Vec<ContentBlock>` round-trips through the
/// text-fallback path for unrepresentable parts.
///
/// Upstream-specific converters (`claude_agent_sdk`, `codex_sdk`) use
/// stricter `TryFrom` impls that reject unsupported parts rather than
/// fall back to text — this `From` is the generic, lossless-or-textual
/// path used by `agent/completions/notify` and similar surfaces.
impl From<crate::agent::completions::message::RichContentPart>
    for ContentBlock
{
    fn from(
        part: crate::agent::completions::message::RichContentPart,
    ) -> Self {
        use crate::agent::completions::message::RichContentPart;
        match part {
            RichContentPart::Text { text } => {
                ContentBlock::Text(super::TextContent {
                    text,
                    annotations: None,
                    _meta: None,
                })
            }
            RichContentPart::ImageUrl { image_url } => {
                // Preserve the URL for the fallback case without
                // double-allocating in the success path.
                let fallback_url = image_url.url.clone();
                match super::ImageContent::try_from(image_url) {
                    Ok(ic) => ContentBlock::Image(ic),
                    Err(_) => ContentBlock::Text(super::TextContent {
                        text: fallback_url,
                        annotations: None,
                        _meta: None,
                    }),
                }
            }
            RichContentPart::InputAudio { input_audio } => {
                ContentBlock::Audio(input_audio.into())
            }
            other @ (RichContentPart::InputVideo { .. }
            | RichContentPart::VideoUrl { .. }
            | RichContentPart::File { .. }) => {
                ContentBlock::Text(super::TextContent {
                    text: serde_json::to_string(&other).unwrap_or_default(),
                    annotations: None,
                    _meta: None,
                })
            }
        }
    }
}

/// Flatten a `RichContent` into the MCP `Vec<ContentBlock>` shape used
/// by `POST /notify` and tool results. `RichContent::Text` yields a
/// single text block; `RichContent::Parts` delegates per-element to
/// [`From<RichContentPart>`].
impl From<crate::agent::completions::message::RichContent>
    for Vec<ContentBlock>
{
    fn from(
        content: crate::agent::completions::message::RichContent,
    ) -> Self {
        use crate::agent::completions::message::RichContent;
        match content {
            RichContent::Text(text) => {
                vec![ContentBlock::Text(super::TextContent {
                    text,
                    annotations: None,
                    _meta: None,
                })]
            }
            RichContent::Parts(parts) => {
                parts.into_iter().map(Into::into).collect()
            }
        }
    }
}
