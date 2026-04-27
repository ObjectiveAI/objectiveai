//! Annotations for content blocks, providing clients additional context.

use serde::{Deserialize, Serialize};

/// The role of a message sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Optional annotations providing clients additional context about content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotations {
    /// Intended audience(s) for the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    /// Importance hint, from 0 (least) to 1 (most).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
    /// ISO 8601 timestamp for the most recent modification.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastModified")]
    pub last_modified: Option<String>,
}
