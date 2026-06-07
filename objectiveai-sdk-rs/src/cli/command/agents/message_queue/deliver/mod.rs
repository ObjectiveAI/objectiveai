//! `agents message-queue deliver` â€” fan out `agents message`
//! against every BOUND target with pending queue rows under the
//! caller's hierarchy (inclusive + recursive), in parallel.
//! Streams the inner `agents message` items merged across all
//! fanned-out deliveries, each augmented with the resolved
//! target's `agent_instance_hierarchy` and (when the row was
//! Tag-addressed) the `agent_tag`.
//!
//! PENDING / ABSENT tag rows are skipped â€” they don't resolve to
//! a spawned target, so there's nothing to deliver to yet. The
//! caller can revisit later, once the tag is BOUND, and the
//! deliver sweep will pick it up.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.deliver.Request")]
pub struct Request {
    pub path_type: Path,
    /// Subtree root to deliver under. Defaults to the cli's own
    /// `Config.agent_instance_hierarchy` when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub parent_agent_instance_hierarchy: Option<String>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.deliver.Path")]
pub enum Path {
    #[serde(rename = "agents/message-queue/deliver")]
    AgentsMessageQueueDeliver,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "message-queue".to_string(),
            "deliver".to_string(),
        ];
        if let Some(p) = &self.parent_agent_instance_hierarchy {
            argv.push(p.clone());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// One item from one of the fanned-out `agents message` calls,
/// augmented with the resolved target that produced it.
///
/// `item` is the inner `agents::message::ResponseItem` â€”
/// `Queued` / `Delivered` / `Chunk`. `agent_instance_hierarchy`
/// is the resolved hierarchy the delivery was addressed to;
/// `agent_tag` is `Some` only when the underlying queue row was
/// Tag-addressed (with the tag now resolved to that hierarchy
/// via the BOUND lookup).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.deliver.ResponseItem")]
pub struct ResponseItem {
    pub agent_instance_hierarchy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_tag: Option<String>,
    pub item: super::super::instances::message::ResponseItem,
}

#[derive(clap::Args)]
pub struct Args {
    /// Subtree root to deliver under. Omit to use the cli's own
    /// position (`Config.agent_instance_hierarchy`).
    pub parent_agent_instance_hierarchy: Option<String>,
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
            path_type: Path::AgentsMessageQueueDeliver,
            parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
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
        // Delegate to the inner `agents::message::ResponseItem`'s
        // own MCP projection so chunks / queued / delivered each
        // ride through the same shape they would on `agents
        // message`. The outer attribution travels as JSONL through
        // the JSONL arms anyway; the Chunk arm projects as media-
        // adjacent JSONL like agents message does today.
        self.item.into_mcp()
    }
}

pub mod request_schema;

pub mod response_schema;
