//! Icon types for MCP entities.

use serde::{Deserialize, Serialize};

/// Theme preference for an icon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IconTheme {
    Light,
    Dark,
}

/// An icon that can be displayed in a user interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Icon {
    /// URL or data URI for the icon.
    pub src: String,
    /// MIME type for the icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    /// Sizes at which the icon can be used (e.g., "48x48", "96x96", "any").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sizes: Option<Vec<String>>,
    /// Theme this icon is intended for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<IconTheme>,
}
