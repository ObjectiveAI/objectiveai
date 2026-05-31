//! `SystemMessageLog` — on-disk shape of [`super::SystemMessage`].
//! `content` is replaced by [`super::SimpleContentLog`] (extracted-to-files);
//! all other fields stay inline.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::SimpleContentLog;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.message.SystemMessageLog")]
pub struct SystemMessageLog {
    pub content: SimpleContentLog,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
}
