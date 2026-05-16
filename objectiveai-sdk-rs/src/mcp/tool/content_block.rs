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
