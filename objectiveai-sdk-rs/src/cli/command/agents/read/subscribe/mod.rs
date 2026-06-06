//! `agents read subscribe` — async handler stub.

use crate::cli::command::CommandRequest;

/// The six values stored in the `messages.kind` TEXT column. Owning
/// this enum in the SDK lets bare-naked callers reason about message
/// kinds without depending on the CLI's filesystem layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[clap(rename_all = "kebab-case")]
#[schemars(rename = "cli.command.agents.read.subscribe.RequestMessageKind")]
pub enum RequestMessageKind {
    AgentCompletionRequest,
    FunctionExecutionRequest,
    FunctionInventionRecursiveRequest,
    AgentCompletionNotification,
    AssistantResponse,
    ToolResponse,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.read.subscribe.Request")]
pub struct Request {
    pub path_type: Path,
    /// Lineage prefix to prepend to [`Self::agent_instance`]. When
    /// `None`, the CLI substitutes its own
    /// `Config.agent_instance_hierarchy` (the cli's "caller"
    /// position). Full target lineage is `"{parent}/{instance}"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub parent_agent_instance_hierarchy: Option<String>,
    /// Leaf id of the target agent. Combined with
    /// [`Self::parent_agent_instance_hierarchy`] (or the cli's
    /// caller position when that is `None`) to form the full
    /// hierarchy.
    pub agent_instance: String,
    pub kind: Option<RequestMessageKind>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.read.subscribe.Path")]
pub enum Path {
    #[serde(rename = "agents/read/subscribe")]
    AgentsReadSubscribe,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "read".to_string(),
            "subscribe".to_string(),
            self.agent_instance.clone(),
        ];
        if let Some(parent) = &self.parent_agent_instance_hierarchy {
            argv.push("--parent-agent-instance-hierarchy".to_string());
            argv.push(parent.clone());
        }
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
        RequestMessageKind::FunctionExecutionRequest => "function-execution-request",
        RequestMessageKind::FunctionInventionRecursiveRequest => {
            "function-invention-recursive-request"
        }
        RequestMessageKind::AgentCompletionNotification => "agent-completion-notification",
        RequestMessageKind::AssistantResponse => "assistant-response",
        RequestMessageKind::ToolResponse => "tool-response",
    }
}

// Share the queue-item / queue-message / content shapes with
// `agents read all` — same on-disk persistence rows surfaced
// through different read patterns.
pub use super::all::{ResponseContent, ResponseQueueItem, ResponseQueueMessage};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.read.subscribe.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Items")]
    Items {
        agent_id: String,
        items: Vec<ResponseQueueItem>,
    },
    #[schemars(title = "Inactive")]
    Inactive {
        agent_id: String,
    },
}

#[derive(clap::Args)]
pub struct Args {
    /// Leaf id of the target agent. Combined with `--parent` (or
    /// the cli's own `Config.agent_instance_hierarchy` when
    /// `--parent` is omitted) to form the full lineage.
    pub agent_instance: String,
    /// Optional lineage prefix to prepend to `agent_instance`.
    /// When omitted, the cli substitutes its own
    /// `Config.agent_instance_hierarchy`.
    #[arg(long = "parent-agent-instance-hierarchy")]
    pub parent_agent_instance_hierarchy: Option<String>,
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
            path_type: Path::AgentsReadSubscribe,
            parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
            agent_instance: args.agent_instance,
            kind: args.kind,
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
