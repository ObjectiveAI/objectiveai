//! `QueueItem` — typed shape of a single row returned by
//! [`crate::filesystem::Client::read_new_from_queue`]. One variant per
//! [`objectiveai_sdk::cli::command::agents::read::subscribe::RequestMessageKind`].

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Content, QueueMessage};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "filesystem.logs.queue.QueueItem")]
pub enum QueueItem {
    #[schemars(title = "AssistantResponse")]
    AssistantResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        reasoning: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        tool_calls: Option<Vec<i64>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        content: Option<Content>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        refusal: Option<i64>,
    },
    #[schemars(title = "ToolResponse")]
    ToolResponse {
        tool_call_id: String,
        content: Content,
    },
    #[schemars(title = "Notification")]
    Notification { content: Content },
    #[schemars(title = "AgentCompletionRequest")]
    AgentCompletionRequest { messages: Vec<QueueMessage> },
    #[schemars(title = "FunctionExecutionRequest")]
    FunctionExecutionRequest { id: i64 },
    #[schemars(title = "FunctionInventionRecursiveRequest")]
    FunctionInventionRecursiveRequest { id: i64 },
}
