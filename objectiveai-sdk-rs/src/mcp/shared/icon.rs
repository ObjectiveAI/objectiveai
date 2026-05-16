//! Icon types for MCP entities.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Theme preference for an icon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "mcp.shared.IconTheme")]
pub enum IconTheme {
    #[schemars(title = "Light")]
    Light,
    #[schemars(title = "Dark")]
    Dark,
}

/// An icon that can be displayed in a user interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.shared.Icon")]
pub struct Icon {
    /// URL or data URI for the icon.
    pub src: String,
    /// MIME type for the icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    /// Sizes at which the icon can be used (e.g., "48x48", "96x96", "any").
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub sizes: Option<Vec<String>>,
    /// Theme this icon is intended for.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub theme: Option<IconTheme>,
}
