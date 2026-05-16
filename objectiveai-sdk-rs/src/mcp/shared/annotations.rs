//! Annotations for content blocks, providing clients additional context.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The role of a message sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "mcp.shared.Role")]
pub enum Role {
    #[schemars(title = "User")]
    User,
    #[schemars(title = "Assistant")]
    Assistant,
}

/// Optional annotations providing clients additional context about content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.shared.Annotations")]
pub struct Annotations {
    /// Intended audience(s) for the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub audience: Option<Vec<Role>>,
    /// Importance hint, from 0 (least) to 1 (most).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub priority: Option<f64>,
    /// ISO 8601 timestamp for the most recent modification.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "lastModified")]
    pub last_modified: Option<String>,
}
