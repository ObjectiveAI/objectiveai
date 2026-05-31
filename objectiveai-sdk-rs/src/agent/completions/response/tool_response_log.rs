//! On-disk shape of a `ToolResponse` log file.
//!
//! Mirrors [`super::ToolResponse`]'s flattened shape (`role`, `index`,
//! then `ToolMessage`'s fields hoisted via `serde(flatten)`). One
//! type swap: `content: RichContent` → `RichContentLog` so media
//! parts can be replaced by references.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::completions::message::{RichContentLog, ToolResponseMetadata};

use super::ToolRole;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.response.ToolResponseLog")]
pub struct ToolResponseLog {
    pub role: ToolRole,
    pub index: u64,
    pub content: RichContentLog,
    pub tool_call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub metadata: Option<ToolResponseMetadata>,
}
