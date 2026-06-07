//! `ToolMessageLog` — postgres-log shape of [`super::ToolMessage`].
//!
//! `content` is a [`RichContentLogRef`] — tool results can be rich
//! (image-bearing). `tool_call_id` stays inline; `metadata` (if
//! present) stays inline too.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ToolResponseMetadata;
use crate::logs::RichContentLogRef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.message.ToolMessageLog")]
pub struct ToolMessageLog {
    pub content: RichContentLogRef,
    pub tool_call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub metadata: Option<ToolResponseMetadata>,
}
