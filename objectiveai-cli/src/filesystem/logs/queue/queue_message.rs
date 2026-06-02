//! `QueueMessage` — per-role message shape inside a
//! [`super::QueueItem::UserRequest`]'s `messages` list. Mirrors the
//! per-role `*MessageLog` types with content/refusal/reasoning/tool_calls
//! flattened to bare integer file-id refs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Content;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "filesystem.logs.queue.QueueMessage")]
pub enum QueueMessage {
    #[schemars(title = "Developer")]
    Developer {
        content: Content,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
    },
    #[schemars(title = "System")]
    System {
        content: Content,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
    },
    #[schemars(title = "User")]
    User {
        content: Content,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
    },
    #[schemars(title = "Assistant")]
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        content: Option<Content>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        reasoning: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        tool_calls: Option<Vec<i64>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        refusal: Option<i64>,
    },
    #[schemars(title = "Tool")]
    Tool {
        content: Content,
        tool_call_id: String,
    },
}
