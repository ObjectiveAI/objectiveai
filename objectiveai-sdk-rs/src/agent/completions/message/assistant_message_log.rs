//! `AssistantMessageLog` — postgres-log shape of
//! [`super::AssistantMessage`].
//!
//! - `content` — rich-content ref (None when the assistant produced
//!   no content, e.g. tool-only turn).
//! - `refusal`, `reasoning` — plain strings, each a `text` table ref.
//! - `tool_calls` — full tool-call shells (id, type, function name)
//!   with the JSON `arguments` extracted to a `text` table ref.
//! - `name` — stays inline.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::AssistantToolCallType;
use crate::logs::{LogRef, RichContentLogRef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.message.AssistantMessageLog")]
pub struct AssistantMessageLog {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub content: Option<RichContentLogRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub refusal: Option<LogRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tool_calls: Option<Vec<AssistantToolCallLog>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<LogRef>,
}

/// One tool call in an assistant message, with `arguments` extracted
/// to the `text` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.completions.message.AssistantToolCallLog")]
pub enum AssistantToolCallLog {
    Function {
        id: String,
        function: AssistantToolCallFunctionLog,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.message.AssistantToolCallFunctionLog")]
pub struct AssistantToolCallFunctionLog {
    pub name: String,
    /// JSON args string → `text` table ref.
    pub arguments: LogRef,
}

impl AssistantToolCallLog {
    /// The constant call-type discriminator (`"function"`). Kept for
    /// callers that need it without matching the enum.
    pub fn call_type(&self) -> AssistantToolCallType {
        match self {
            AssistantToolCallLog::Function { .. } => AssistantToolCallType::Function,
        }
    }
}
