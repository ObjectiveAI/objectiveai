//! MCP content block enum.
//!
//! A content block is the union of all content types that can appear in
//! prompts, tool results, and sampling messages.

use serde::{Deserialize, Serialize};

/// A content block that can be used in prompts and tool results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Text content.
    #[serde(rename = "text")]
    Text(super::TextContent),
    /// Image content (base64-encoded).
    #[serde(rename = "image")]
    Image(super::ImageContent),
    /// Audio content (base64-encoded).
    #[serde(rename = "audio")]
    Audio(super::AudioContent),
    /// A resource link.
    #[serde(rename = "resource_link")]
    ResourceLink(super::ResourceLink),
    /// An embedded resource.
    #[serde(rename = "resource")]
    EmbeddedResource(super::EmbeddedResource),
}
