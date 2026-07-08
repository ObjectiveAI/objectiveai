//! `agents logs token-usage subscribe` — wait for an agent's stored
//! `total_tokens` snapshot to change, OR for its instance lock to
//! drop. One-shot: yields exactly one result — a token-usage value
//! (`Item`) or the literal string `"agents_inactive"` — then ends.
//!
//! With `--previous`, returns immediately when the stored value
//! already differs; otherwise it subscribes.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.token_usage.subscribe.Request")]
pub struct Request {
    pub path_type: Path,
    /// The full agent instance hierarchy to watch.
    pub agent_instance_hierarchy: String,
    /// The last-seen `total_tokens`. When set and the stored value
    /// already differs, the command returns immediately instead of
    /// subscribing. `None` on the wire = always subscribe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub previous: Option<i64>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.token_usage.subscribe.Path")]
pub enum Path {
    #[serde(rename = "agents/logs/token-usage/subscribe")]
    AgentsLogsTokenUsageSubscribe,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Subscribe's wire shape: either a token-usage snapshot (`Item`) or
/// the literal string `"agents_inactive"`. `#[serde(untagged)]` so
/// `Item` passes through as a plain object and `AgentsInactive` as a
/// bare string — the same convention as `agents logs subscribe`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.logs.token_usage.subscribe.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Item")]
    Item(TokenUsage),
    #[schemars(title = "AgentsInactive")]
    AgentsInactive(AgentsInactiveTag),
}

/// A per-AIH token-usage snapshot.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.token_usage.subscribe.TokenUsage")]
pub struct TokenUsage {
    pub agent_instance_hierarchy: String,
    pub total_tokens: i64,
}

/// Single-variant enum whose lone variant serializes as the literal
/// string `"agents_inactive"` (mirrors `logs::subscribe::AgentsInactiveTag`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.token_usage.subscribe.AgentsInactiveTag")]
pub enum AgentsInactiveTag {
    #[serde(rename = "agents_inactive")]
    AgentsInactive,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("aih_required").required(true).args(["agent_instance_hierarchy"])))]
pub struct Args {
    /// The full agent instance hierarchy to watch.
    #[arg(long = "agent-instance-hierarchy")]
    pub agent_instance_hierarchy: Option<String>,
    /// Last-seen `total_tokens`; return immediately if the stored value
    /// already differs.
    #[arg(long)]
    pub previous: Option<i64>,
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
        Ok(Self {
            path_type: Path::AgentsLogsTokenUsageSubscribe,
            agent_instance_hierarchy: args.agent_instance_hierarchy.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "agent-instance-hierarchy",
                    "--agent-instance-hierarchy is required".to_string(),
                )
            })?,
            previous: args.previous,
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

/// One `/listen` broadcast run of `agents logs token_usage subscribe`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// response-item stream. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::websocket_listener::ResponseItemStream<ResponseItem>,
}
