//! `laboratories connect` — spawn the resident manager that connects a
//! CREATED laboratory to a daemon: the local one by default, or any
//! remote daemon via `--address`. The manager holds the per-state
//! `(id, address)` connection lock, starts the container, and serves
//! MCP + transfer traffic over the daemon's `/laboratory` route until
//! killed (stopping the container on graceful shutdown).

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.connect.Request")]
pub struct Request {
    pub path_type: Path,
    /// The laboratory id (must already be created).
    pub id: String,
    /// The daemon `ws://` base address to connect to. `None` = the
    /// LOCAL daemon (ensured + resolved by the CLI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub address: Option<String>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.connect.Path")]
pub enum Path {
    #[serde(rename = "laboratories/connect")]
    LaboratoriesConnect,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Confirmation — the manager is spawned (and, for a local daemon,
/// CONNECTED). Echoes the id and the RESOLVED daemon address.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.connect.Response")]
pub struct Response {
    pub id: String,
    /// The daemon address the manager dials (the local daemon's
    /// published `ws://` URL when the request left `address` unset).
    pub address: String,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("id_required").required(true).args(["id"])))]
pub struct Args {
    /// The laboratory id (must already be created).
    #[arg(long)]
    pub id: Option<String>,
    /// Daemon `ws://` base address to connect to; omit for the local
    /// daemon.
    #[arg(long)]
    pub address: Option<String>,
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
        let id = args.id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "id",
                "--id is required".to_string(),
            )
        })?;
        Ok(Self {
            path_type: Path::LaboratoriesConnect,
            id,
            address: args.address,
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

/// One `/listen` broadcast run of `laboratories connect`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::websocket_listener::UnaryResponse<Response>,
}
