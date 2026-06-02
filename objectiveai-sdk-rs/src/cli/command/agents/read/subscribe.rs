//! `agents read subscribe` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequestMessageKind {
    AgentCompletionRequest,
    AgentCompletionResponse,
    AgentCompletionMessage,
    AssistantResponse,
    ContinuationToken,
    Sweep,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub agent_instance_hierarchy: String,
    pub kind: Option<RequestMessageKind>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "read".to_string(),
            "subscribe".to_string(),
            self.agent_instance_hierarchy.clone(),
        ];
        if let Some(kind) = &self.kind {
            argv.push("--kind".to_string());
            argv.push(message_kind_flag(kind).to_string());
        }
        argv
    }
}

fn message_kind_flag(kind: &RequestMessageKind) -> &'static str {
    // Wire form matches clap's `value_enum` rename_all = "kebab-case" default.
    match kind {
        RequestMessageKind::AgentCompletionRequest => "agent-completion-request",
        RequestMessageKind::AgentCompletionResponse => "agent-completion-response",
        RequestMessageKind::AgentCompletionMessage => "agent-completion-message",
        RequestMessageKind::AssistantResponse => "assistant-response",
        RequestMessageKind::ContinuationToken => "continuation-token",
        RequestMessageKind::Sweep => "sweep",
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
pub enum ResponseItem {
    Items {
        agent_id: String,
        items: Vec<ResponseQueueItem>,
    },
    Inactive {
        agent_id: String,
    },
}
