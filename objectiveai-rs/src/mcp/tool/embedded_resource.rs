//! Embedded resource content block.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The contents of a resource, embedded into a prompt or tool call result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedResource {
    /// The embedded resource contents.
    pub resource: super::super::shared::ResourceContentsUnion,
    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<super::super::shared::Annotations>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
