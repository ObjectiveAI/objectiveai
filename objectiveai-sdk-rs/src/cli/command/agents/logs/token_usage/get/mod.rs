//! `agents logs token-usage get` — read an agent's current stored
//! `total_tokens` snapshot (no waiting). `total_tokens` is null when no
//! agent-completion usage has been recorded for the AIH yet.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.token_usage.get.Request")]
pub struct Request {
    pub path_type: Path,
    /// The full agent instance hierarchy to read.
    pub agent_instance_hierarchy: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.token_usage.get.Path")]
pub enum Path {
    #[serde(rename = "agents/logs/token-usage/get")]
    AgentsLogsTokenUsageGet,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The current stored snapshot. `total_tokens` is `None` when no
/// agent-completion usage has been recorded for this AIH yet.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.token_usage.get.Response")]
pub struct Response {
    pub agent_instance_hierarchy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub total_tokens: Option<i64>,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("aih_required").required(true).args(["agent_instance_hierarchy"])))]
pub struct Args {
    /// The full agent instance hierarchy to read.
    #[arg(long = "agent-instance-hierarchy")]
    pub agent_instance_hierarchy: Option<String>,
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
            path_type: Path::AgentsLogsTokenUsageGet,
            agent_instance_hierarchy: args.agent_instance_hierarchy.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "agent-instance-hierarchy",
                    "--agent-instance-hierarchy is required".to_string(),
                )
            })?,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    identity: Option<&crate::identity::Identity>,
) -> Result<Response, E::Error> {
    request.base.clear_transform();
    executor.execute_one(request, identity).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    identity: Option<&crate::identity::Identity>,
) -> Result<serde_json::Value, E::Error> {
    request.base.set_transform(transform);
    executor.execute_one(request, identity).await
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `agents logs token_usage get`: the actual
/// [`Request`], the producer's
/// [`Identity`](crate::identity::Identity), and the
/// unary response future. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
