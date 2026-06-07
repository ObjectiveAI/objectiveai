//! `UserMessageLog` — postgres-log shape of [`super::UserMessage`].
//!
//! `content` is a [`RichContentLogRef`] — solo text ref or rich-media
//! list. Everything else stays inline.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logs::RichContentLogRef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.message.UserMessageLog")]
pub struct UserMessageLog {
    pub content: RichContentLogRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
}
