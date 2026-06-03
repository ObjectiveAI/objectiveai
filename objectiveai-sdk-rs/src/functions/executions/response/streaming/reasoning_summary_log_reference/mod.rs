//! `LogReference` for the reasoning-summary entry within a
//! function-execution log file. Carries the wrapper's own
//! top-level `error` so the summary's failure surface is visible at
//! the parent's reference level (the inner agent-completion's own
//! error is preserved separately in its log file).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logs::LogReferenceTag;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.executions.response.streaming.reasoning_summary_log_reference.LogReference"
)]
pub struct LogReference {
    #[serde(rename = "type")]
    pub r#type: LogReferenceTag,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<serde_json::Value>,
}

impl LogReference {
    pub fn new(path: String) -> Self {
        Self {
            r#type: LogReferenceTag::Reference,
            path,
            error: None,
        }
    }
}
