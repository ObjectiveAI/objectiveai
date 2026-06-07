//! `ToolResponseLog` — postgres-log shape of [`super::ToolResponse`].
//!
//! Mirrors the wire-side fields with one swap: `content: RichContent`
//! → `RichContentLogRef` so each content part lands in its matching
//! media table.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::completions::message::ToolResponseMetadata;
use crate::logs::RichContentLogRef;

use super::ToolRole;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.response.ToolResponseLog")]
pub struct ToolResponseLog {
    pub role: ToolRole,
    pub index: u64,
    pub content: RichContentLogRef,
    pub tool_call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub metadata: Option<ToolResponseMetadata>,
}
