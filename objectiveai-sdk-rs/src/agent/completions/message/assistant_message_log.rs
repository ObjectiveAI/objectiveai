//! `AssistantMessageLog` — on-disk shape of [`super::AssistantMessage`].
//! `content` becomes `Option<RichContentLog>` (extracted-to-files when
//! present); `refusal`, `reasoning`, and each `tool_call` are also
//! extracted to their own files and referenced — only `name` stays
//! inline. Mirrors the response-side
//! [`crate::agent::completions::response::streaming::AssistantResponseChunkLog`].

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::RichContentLog;
use crate::logs::LogReference;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.message.AssistantMessageLog")]
pub struct AssistantMessageLog {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub content: Option<RichContentLog>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub refusal: Option<LogReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tool_calls: Option<Vec<LogReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<LogReference>,
}
