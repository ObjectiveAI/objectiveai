//! `agents laboratories detach` — detach a laboratory id from an agent
//! target (a tag, or an instance hierarchy via PAIH + `--agent-instance`).
//! Keyed the same way as `attach`; see the CLI handler for the lock +
//! delete. Errors if the laboratory was not attached.

use crate::cli::command::CommandRequest;
use crate::cli::command::agents::selector::{AgentSelector, AgentSelectorArgs};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.laboratories.detach.Request")]
pub struct Request {
    pub path_type: Path,
    pub selector: AgentSelector,
    pub laboratory_id: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.laboratories.detach.Path")]
pub enum Path {
    #[serde(rename = "agents/laboratories/detach")]
    AgentsLaboratoriesDetach,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Empty confirmation — detach succeeded.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.laboratories.detach.Response")]
pub struct Response {}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("laboratory_id_required").required(true).args(["laboratory_id"])))]
pub struct Args {
    #[command(flatten)]
    pub selector: AgentSelectorArgs,
    /// Laboratory id to detach from the target agent.
    #[arg(long)]
    pub laboratory_id: Option<String>,
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
        let selector = AgentSelector::try_from(args.selector)?;
        let laboratory_id = args.laboratory_id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "laboratory_id",
                "--laboratory-id is required".to_string(),
            )
        })?;
        Ok(Self {
            path_type: Path::AgentsLaboratoriesDetach,
            selector,
            laboratory_id,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<Response, E::Error> {
    request.base.clear_transform();
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<serde_json::Value, E::Error> {
    request.base.set_transform(transform);
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;
