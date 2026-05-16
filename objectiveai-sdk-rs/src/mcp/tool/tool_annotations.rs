//! Tool annotation metadata.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Additional metadata about a tool to help clients decide how to display
/// or control its use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.tool.ToolAnnotations")]
pub struct ToolAnnotations {
    /// A human-readable title for the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub title: Option<String>,
    /// If true, the tool does not modify its environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "readOnlyHint")]
    pub read_only_hint: Option<bool>,
    /// If true, the tool may perform destructive updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "destructiveHint")]
    pub destructive_hint: Option<bool>,
    /// If true, calling the tool repeatedly with the same arguments
    /// has no additional effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "idempotentHint")]
    pub idempotent_hint: Option<bool>,
    /// If true, the tool interacts with the external world.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "openWorldHint")]
    pub open_world_hint: Option<bool>,
}
