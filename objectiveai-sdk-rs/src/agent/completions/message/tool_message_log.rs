//! `ToolMessageLog` — on-disk shape of [`super::ToolMessage`].
//! `content` is replaced by [`super::RichContentLog`] (extracted-to-files);
//! tool_call_id + metadata stay inline.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{RichContentLog, ToolResponseMetadata};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.message.ToolMessageLog")]
pub struct ToolMessageLog {
    pub content: RichContentLog,
    pub tool_call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub metadata: Option<ToolResponseMetadata>,
}
