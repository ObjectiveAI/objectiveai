//! `agents read subscribe` â€” async handler stub.

use crate::cli::command::CommandRequest;

/// The six values stored in the `messages.kind` TEXT column. Owning
/// this enum in the SDK lets bare-naked callers reason about message
/// kinds without depending on the CLI's filesystem layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[clap(rename_all = "kebab-case")]
#[schemars(rename = "cli.command.agents.instances.read.subscribe.RequestMessageKind")]
pub enum RequestMessageKind {
    AgentCompletionRequest,
    FunctionExecutionRequest,
    FunctionInventionRecursiveRequest,
    AgentCompletionNotification,
    AssistantResponse,
    ToolResponse,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.read.subscribe.Request")]
pub struct Request {
    pub path_type: Path,
    pub target: SubscribeTarget,
    pub kind: Option<RequestMessageKind>,
    pub jq: Option<String>,
}

/// Mutually-exclusive target selector: either direct `(parent,
/// instance)` addressing (with the parent defaulting to the cli's
/// own `Config.agent_instance_hierarchy` when omitted) OR a tag name
/// the cli resolves at handler time.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.instances.read.subscribe.SubscribeTarget")]
pub enum SubscribeTarget {
    #[schemars(title = "Direct")]
    Direct {
        /// Lineage prefix to prepend to `agent_instance`. When
        /// `None`, the CLI substitutes its own
        /// `Config.agent_instance_hierarchy`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
        /// Leaf id of the target agent.
        agent_instance: String,
    },
    #[schemars(title = "Tag")]
    Tag { agent_tag: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.read.subscribe.Path")]
pub enum Path {
    #[serde(rename = "agents/instances/read/subscribe")]
    AgentsInstancesReadSubscribe,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "instances".to_string(),
            "read".to_string(),
            "subscribe".to_string(),
        ];
        match &self.target {
            SubscribeTarget::Direct {
                parent_agent_instance_hierarchy,
                agent_instance,
            } => {
                argv.push(agent_instance.clone());
                if let Some(parent) = parent_agent_instance_hierarchy {
                    argv.push("--parent-agent-instance-hierarchy".to_string());
                    argv.push(parent.clone());
                }
            }
            SubscribeTarget::Tag { agent_tag } => {
                argv.push("--agent-tag".to_string());
                argv.push(agent_tag.clone());
            }
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
// `agents read all` â€” same on-disk persistence rows surfaced
// through different read patterns.
pub use super::all::{ResponseContent, ResponseQueueItem, ResponseQueueMessage};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.instances.read.subscribe.ResponseItem")]
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
#[command(group(
    clap::ArgGroup::new("subscribe_target")
        .required(true)
        .multiple(false)
        .args(["agent_instance", "agent_tag"])
))]
pub struct Args {
    /// Leaf id of the target agent. Combined with `--parent` (or
    /// the cli's own `Config.agent_instance_hierarchy` when
    /// `--parent` is omitted) to form the full lineage. Mutually
    /// exclusive with `--agent-tag`.
    pub agent_instance: Option<String>,
    /// Optional lineage prefix to prepend to `agent_instance`.
    /// When omitted, the cli substitutes its own
    /// `Config.agent_instance_hierarchy`. Only valid alongside a
    /// positional `agent_instance` (mutually exclusive with
    /// `--agent-tag`).
    #[arg(long = "parent-agent-instance-hierarchy", requires = "agent_instance")]
    pub parent_agent_instance_hierarchy: Option<String>,
    /// Resolve the target via a previously-bound tag. Mutually
    /// exclusive with `agent_instance` and
    /// `--parent-agent-instance-hierarchy`.
    #[arg(long = "agent-tag")]
    pub agent_tag: Option<String>,
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
        let target = match (args.agent_instance, args.agent_tag) {
            (Some(agent_instance), None) => SubscribeTarget::Direct {
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
                agent_instance,
            },
            (None, Some(agent_tag)) => SubscribeTarget::Tag { agent_tag },
            _ => unreachable!(
                "clap group `subscribe_target` ensures exactly one of agent_instance | agent_tag"
            ),
        };
        Ok(Self {
            path_type: Path::AgentsInstancesReadSubscribe,
            target,
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
