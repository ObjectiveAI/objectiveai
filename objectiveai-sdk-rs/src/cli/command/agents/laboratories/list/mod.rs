//! `agents laboratories list` — stream the laboratory ids attached to an
//! agent target (a tag, or an instance hierarchy via PAIH +
//! `--agent-instance`). Read-only; see the CLI handler. Each item is
//! `{ "id": "<laboratory id>" }`.

use crate::cli::command::CommandRequest;
use crate::cli::command::agents::selector::{AgentSelector, AgentSelectorArgs};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.laboratories.list.Request")]
pub struct Request {
    pub path_type: Path,
    pub selector: AgentSelector,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.laboratories.list.Path")]
pub enum Path {
    #[serde(rename = "agents/laboratories/list")]
    AgentsLaboratoriesList,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// One attached laboratory id.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.laboratories.list.ResponseItem")]
pub struct ResponseItem {
    pub id: String,
}

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub selector: AgentSelectorArgs,
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
    /// Emit the JSON Schema for this leaf's `ResponseItem` type and exit.
    ResponseSchema(response_schema::Args),
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        let selector = AgentSelector::try_from(args.selector)?;
        Ok(Self {
            path_type: Path::AgentsLaboratoriesList,
            selector,
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

/// One `/listen` broadcast run of `agents laboratories list`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// response-item stream. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::websocket_listener::ResponseItemStream<ResponseItem>,
}
