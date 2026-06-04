//! `agents read all` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.read.all.Request")]
pub struct Request {
    pub path_type: Path,
    pub agent_instance_hierarchies: Vec<String>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.read.all.Path")]
pub enum Path {
    #[serde(rename = "agents/read/all")]
    AgentsReadAll,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "read".to_string(),
            "all".to_string(),
        ];
        argv.extend(self.agent_instance_hierarchies.iter().cloned());
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.read.all.ResponseContent")]
pub enum ResponseContent {
    #[schemars(title = "One")]
    One(i64),
    #[schemars(title = "Many")]
    Many(Vec<i64>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.read.all.ResponseQueueMessage")]
pub enum ResponseQueueMessage {
    #[schemars(title = "Developer")]
    Developer {
        content: ResponseContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[schemars(title = "System")]
    System {
        content: ResponseContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[schemars(title = "User")]
    User {
        content: ResponseContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[schemars(title = "Assistant")]
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
    #[schemars(title = "Tool")]
    Tool {
        content: ResponseContent,
        tool_call_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.read.all.ResponseQueueItem")]
pub enum ResponseQueueItem {
    #[schemars(title = "AssistantResponse")]
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
    #[schemars(title = "ToolResponse")]
    ToolResponse {
        tool_call_id: String,
        content: ResponseContent,
    },
    #[schemars(title = "Notification")]
    Notification {
        content: ResponseContent,
    },
    #[schemars(title = "AgentCompletionRequest")]
    AgentCompletionRequest {
        messages: Vec<ResponseQueueMessage>,
    },
    #[schemars(title = "FunctionExecutionRequest")]
    FunctionExecutionRequest {
        id: i64,
    },
    #[schemars(title = "FunctionInventionRecursiveRequest")]
    FunctionInventionRecursiveRequest {
        id: i64,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.read.all.ResponseItem")]
pub struct ResponseItem {
    pub agent_id: String,
    pub items: Vec<ResponseQueueItem>,
}

#[derive(clap::Args)]
pub struct Args {
    /// One or more agent_instance_hierarchy values.
    #[arg(required = true)]
    pub agent_instance_hierarchies: Vec<String>,
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
        Ok(Self { path_type: Path::AgentsReadAll,
            agent_instance_hierarchies: args.agent_instance_hierarchies,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.jq = None;
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.jq = Some(jq);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
