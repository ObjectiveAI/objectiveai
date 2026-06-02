//! `agents read pending` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub agent_instance_hierarchies: Vec<String>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "read".to_string(),
            "pending".to_string(),
        ];
        argv.extend(self.agent_instance_hierarchies.iter().cloned());
        argv
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ResponseContent {
    One(i64),
    Many(Vec<i64>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseQueueMessage {
    Developer {
        content: ResponseContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    System {
        content: ResponseContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    User {
        content: ResponseContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ResponseContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<i64>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        refusal: Option<i64>,
    },
    Tool {
        content: ResponseContent,
        tool_call_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseQueueItem {
    AssistantResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<i64>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ResponseContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        refusal: Option<i64>,
    },
    ToolResponse {
        tool_call_id: String,
        content: ResponseContent,
    },
    Notification {
        content: ResponseContent,
    },
    AgentCompletionRequest {
        messages: Vec<ResponseQueueMessage>,
    },
    FunctionExecutionRequest {
        id: i64,
    },
    FunctionInventionRecursiveRequest {
        id: i64,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResponseItem {
    pub agent_id: String,
    pub items: Vec<ResponseQueueItem>,
}
