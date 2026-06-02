//! `agents read subscribe` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[clap(rename_all = "kebab-case")]
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
    pub jq: Option<String>,
}

impl CommandRequest for Request {
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
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
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
#[serde(untagged)]
pub enum ResponseItem {
    Items {
        agent_id: String,
        items: Vec<ResponseQueueItem>,
    },
    Inactive {
        agent_id: String,
    },
}

#[derive(clap::Args)]
pub struct Args {
    /// Lineage path of the agent to subscribe to.
    pub agent_instance_hierarchy: String,
    /// Filter the stream to messages of this kind only.
    #[arg(long, value_enum)]
    pub kind: Option<RequestMessageKind>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Command {
    #[command(flatten)]
    pub args: Args,
    #[command(subcommand)]
    pub schema: Option<Schema>,
}

#[derive(clap::Subcommand)]
pub enum Schema {
    /// Emit the JSON Schema for this leaf's `Request` type and exit.
    RequestSchema(request_schema::Args),
    /// Emit the JSON Schema for this leaf's `Response` type and exit.
    ResponseSchema(response_schema::Args),
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        Ok(Self {
            agent_instance_hierarchy: args.agent_instance_hierarchy,
            kind: args.kind,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.jq = None;
    executor.execute(request).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,
) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.jq = Some(jq);
    executor.execute(request).await
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    #[derive(clap::Args)]
    pub struct Args {
        #[arg(long)]
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["agents", "read", "subscribe", "request-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;

    #[cfg(feature = "cli-executor")]
    pub async fn execute<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
    ) -> Result<Response, E::Error> {
        request.jq = None;
        executor.execute_one(request).await
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
        jq: String,
    ) -> Result<serde_json::Value, E::Error> {
        request.jq = Some(jq);
        executor.execute_one(request).await
    }
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    #[derive(clap::Args)]
    pub struct Args {
        #[arg(long)]
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["agents", "read", "subscribe", "response-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;

    #[cfg(feature = "cli-executor")]
    pub async fn execute<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
    ) -> Result<Response, E::Error> {
        request.jq = None;
        executor.execute_one(request).await
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
        jq: String,
    ) -> Result<serde_json::Value, E::Error> {
        request.jq = Some(jq);
        executor.execute_one(request).await
    }
}
