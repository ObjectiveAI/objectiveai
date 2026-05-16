//! Resource link content block.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A resource link included in a prompt or tool call result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.tool.ResourceLink")]
pub struct ResourceLink {
    /// The programmatic name of the resource.
    pub name: String,
    /// The URI of this resource.
    pub uri: String,
    /// A human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub title: Option<String>,
    /// A description of what this resource represents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    /// Icons for the resource link.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub icons: Option<Vec<super::super::shared::Icon>>,
    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub annotations: Option<super::super::shared::Annotations>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
