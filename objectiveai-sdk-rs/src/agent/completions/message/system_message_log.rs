//! `SystemMessageLog` — postgres-log shape of [`super::SystemMessage`].
//!
//! `content` is a [`RichContentLogRef`] — either one ref into the
//! `text` table, or an ordered list of mixed-media refs. Everything
//! else stays inline.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logs::RichContentLogRef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.message.SystemMessageLog")]
pub struct SystemMessageLog {
    pub content: RichContentLogRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
}
