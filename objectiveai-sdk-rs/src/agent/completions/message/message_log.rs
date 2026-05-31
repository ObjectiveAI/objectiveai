//! `MessageLog` — on-disk shape of [`super::Message`].
//!
//! Mirrors `Message`'s per-role variants but with content fields
//! swapped to their `*Log` substitutes (Simple → SimpleContentLog,
//! Rich → RichContentLog). Tool-call lists and tool-response
//! metadata stay inline.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AssistantMessageLog, DeveloperMessageLog, SystemMessageLog, ToolMessageLog,
    UserMessageLog,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "role")]
#[schemars(rename = "agent.completions.message.MessageLog")]
pub enum MessageLog {
    #[serde(rename = "developer")]
    #[schemars(title = "Developer")]
    Developer(DeveloperMessageLog),
    #[serde(rename = "system")]
    #[schemars(title = "System")]
    System(SystemMessageLog),
    #[serde(rename = "user")]
    #[schemars(title = "User")]
    User(UserMessageLog),
    #[serde(rename = "assistant")]
    #[schemars(title = "Assistant")]
    Assistant(AssistantMessageLog),
    #[serde(rename = "tool")]
    #[schemars(title = "Tool")]
    Tool(ToolMessageLog),
}
