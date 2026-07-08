//! `agents instances list` — enumerate agent instances under one or
//! more targets, reporting per-agent aggregates (tags, queued count,
//! spawn/active timestamps, total logged messages).

use crate::cli::command::CommandRequest;

/// Reuse the shared `--target` enum (`instance=L[,parent=P]`, `tag=T`,
/// `me`) from `agents logs read all`. Each target resolves to an AIH
/// whose direct children this command lists.
pub use super::super::logs::list::Target;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.list.Request")]
pub struct Request {
    pub path_type: Path,
    /// Resolved targets whose direct children are listed. Must be
    /// empty when `all` is set.
    pub targets: Vec<Target>,
    /// List EVERY instance in the state — mutually exclusive with
    /// `targets`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub all: Option<bool>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.list.Path")]
pub enum Path {
    #[serde(rename = "agents/instances/list")]
    AgentsInstancesList,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// One discovered agent instance under a target. Aggregated from the
/// `logs.messages`, `message_queue`, and `tags` tiers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.list.ResponseItem")]
pub struct ResponseItem {
    /// Full hierarchy of this agent instance.
    pub agent_instance_hierarchy: String,
    /// Tag names currently bound to this AIH, newest-bound first.
    pub tags: Vec<String>,
    /// Active `message_queue` rows targeting this agent — counting
    /// both direct-AIH rows and rows whose tag is bound to this AIH.
    pub queued: u64,
    /// RFC3339 timestamp of the first `logs.messages` row for this
    /// agent. `None` when the agent has no logs yet (queue-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<String>,
    /// RFC3339 timestamp of the most recent `logs.messages` row for
    /// this agent. `None` when the agent has no logs yet (queue-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub last_active_at: Option<String>,
    /// Total `logs.messages` rows for this agent over all time.
    pub logged: u64,
    /// The agent definition recorded for this AIH — from
    /// `objectiveai.agent_refs`, with the legacy most-recent-request
    /// fallback (`lookup_session`). Populated by `agents instances
    /// get`; `agents instances list` leaves it unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent: Option<crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional>,
}

#[derive(clap::Args)]
pub struct Args {
    /// One or more `--target instance=L[,parent=P]` entries. Also
    /// accepts `--target tag=T` and `--target me`. Lists the direct
    /// children of each resolved target.
    #[arg(long = "target", required_unless_present = "all")]
    pub targets: Vec<String>,
    /// List EVERY instance in the state. Mutually exclusive with
    /// `--target`.
    #[arg(long = "all", conflicts_with = "targets")]
    pub all: bool,
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
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
        let targets = args
            .targets
            .iter()
            .map(|s| {
                s.parse::<Target>().map_err(|msg| {
                    crate::cli::command::FromArgsError::path_parse("target", msg)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            path_type: Path::AgentsInstancesList,
            targets,
            all: args.all.then_some(true),
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.base.clear_transform();
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.base.set_transform(transform);
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

/// One `/listen` broadcast run of `agents instances list`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// response-item stream. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::websocket_listener::ResponseItemStream<ResponseItem>,
}
