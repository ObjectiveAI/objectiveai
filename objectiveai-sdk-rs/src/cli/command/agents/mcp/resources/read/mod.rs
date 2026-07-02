//! `agents mcp resources read` — run `resources/read` against the
//! per-`response_id` MCP listener socket and return the MCP
//! `ReadResourceResult`. `--params` is the MCP `ReadResourceRequestParams`,
//! supplied as a JSON string (e.g. `{"uri":"..."}`).

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.mcp.resources.read.Request")]
pub struct Request {
    pub path_type: Path,
    pub response_id: String,
    pub params: crate::mcp::resource::ReadResourceRequestParams,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.mcp.resources.read.Path")]
pub enum Path {
    #[serde(rename = "agents/mcp/resources/read")]
    AgentsMcpResourcesRead,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

pub type Response = crate::mcp::resource::ReadResourceResult;

/// Viewer-stream mirror of [`Request`]: the request (nested under
/// `value`, `path_type` and all) plus the broadcast stream `id`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.mcp.resources.read.ViewerRequest")]
pub struct ViewerRequest {
    pub id: String,
    pub value: Request,
}

/// Viewer-stream mirror of [`Response`]: the response (nested under
/// `value`) plus the broadcast stream `id` and the originating request's
/// `path_type`.
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.mcp.resources.read.ViewerResponse")]
pub struct ViewerResponse {
    pub id: String,
    pub path_type: Path,
    pub value: Response,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("response_id_required").required(true).args(["response_id"])))]
#[command(group(clap::ArgGroup::new("params_required").required(true).args(["params"])))]
pub struct Args {
    /// Objectiveai response id of the live agent whose MCP aggregation
    /// to query (the socket at `<state>/socks/<response_id>.sock`).
    #[arg(long)]
    pub response_id: Option<String>,
    /// MCP `ReadResourceRequestParams` as a JSON string, e.g.
    /// `{"uri":"..."}`.
    #[arg(long)]
    pub params: Option<String>,
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
        let response_id = args.response_id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "response_id",
                "--response-id is required".to_string(),
            )
        })?;
        let params = {
            let s = args.params.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "params",
                    "--params is required".to_string(),
                )
            })?;
            let mut de = serde_json::Deserializer::from_str(&s);
            serde_path_to_error::deserialize(&mut de)
                .map_err(|source| crate::cli::command::FromArgsError {
                    field: "params",
                    source: source.into(),
                })?
        };
        Ok(Self {
            path_type: Path::AgentsMcpResourcesRead,
            response_id,
            params,
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
