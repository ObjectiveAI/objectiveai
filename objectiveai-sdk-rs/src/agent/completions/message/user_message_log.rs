//! `UserMessageLog` — on-disk shape of [`super::UserMessage`].
//! `content` is replaced by [`super::RichContentLog`] (extracted-to-files);
//! all other fields stay inline.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::RichContentLog;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.message.UserMessageLog")]
pub struct UserMessageLog {
    pub content: RichContentLog,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
}
