//! `agents mcp resources list` — run `resources/list` against the
//! per-`response_id` MCP listener socket and return the MCP
//! `ListResourcesResult`. `--params` is the MCP `ListResourcesRequest`,
//! supplied as a JSON string (e.g. `{}` or `{"cursor":"..."}`).

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.mcp.resources.list.Request")]
pub struct Request {
    pub path_type: Path,
    /// Objectiveai response id of the live agent to address. `None` ⇒
    /// resolved from the caller's contextual agent arguments
    /// (`OBJECTIVEAI_RESPONSE_ID`); an error if absent there too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_id: Option<String>,
    pub params: crate::mcp::resource::ListResourcesRequest,
    /// Restrict the listing to the single server with this name (the
    /// routing prefix `agents mcp servers list` reports). `None` lists
    /// every server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.mcp.resources.list.Path")]
pub enum Path {
    #[serde(rename = "agents/mcp/resources/list")]
    AgentsMcpResourcesList,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

pub type Response = crate::mcp::resource::ListResourcesResult;

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("params_required").required(true).args(["params"])))]
pub struct Args {
    /// Objectiveai response id of the live agent whose MCP aggregation
    /// to query (the socket at `<state>/socks/<response_id>.sock`).
    /// Omit to use the invoking agent's own response id (from the
    /// contextual agent arguments).
    #[arg(long)]
    pub response_id: Option<String>,
    /// MCP `ListResourcesRequest` as a JSON string, e.g. `{}` or
    /// `{"cursor":"..."}`.
    #[arg(long)]
    pub params: Option<String>,
    /// Restrict the listing to the single server with this name (the
    /// routing prefix from `agents mcp servers list`). Omit to list all.
    #[arg(long)]
    pub name: Option<String>,
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
            path_type: Path::AgentsMcpResourcesList,
            response_id: args.response_id,
            params,
            name: args.name,
            base: args.base.into(),
        })
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

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `agents mcp resources list`: the actual
/// [`Request`], the producer's
/// [`Identity`](crate::identity::Identity), and the
/// unary response future. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
