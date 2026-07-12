//! `laboratories delete` — remove a laboratory container (a podman
//! `rm -f`), reclaiming its disk. Names the container per state
//! (`--id`); `--client` is required (a required arg-group, matching
//! `create`, leaving room to add `--server` later). The leaf echoes
//! back the id it removed.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.delete.Request")]
pub struct Request {
    pub path_type: Path,
    pub kind: Kind,
    pub id: String,
    /// The EXACT machine id whose laboratory host owns the container.
    /// Provided together with `machine_state` or not at all; neither ⇒
    /// the current machine + the daemon's own state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<String>,
    /// The state (on `machine`) whose laboratory host owns the
    /// container. Paired with `machine` — both or neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.delete.Path")]
pub enum Path {
    #[serde(rename = "laboratories/delete")]
    LaboratoriesDelete,
}

/// Which side of the conduit the laboratory serves. Only `Client`
/// exists today; the tag-by-`by` shape leaves room to add `Server`
/// later.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.laboratories.delete.Kind")]
pub enum Kind {
    // No variant-level `#[schemars(title = "...")]`: a single-variant enum
    // collapses and hoists the variant title to the schema's top-level
    // `title`, which the JS codegen then uses as the module path — a title
    // of "Client" would clobber `src/client.ts`. Let `rename` drive it.
    Client,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Echo of the removed laboratory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.delete.Response")]
pub struct Response {
    pub id: String,
}

#[derive(clap::Args)]
#[command(
    group(clap::ArgGroup::new("side").required(true).args(["client"])),
    group(clap::ArgGroup::new("id_required").required(true).args(["id"])),
)]
pub struct Args {
    /// Delete a client-side laboratory (an MCP server the conduit dials).
    /// Required (part of the `side` arg-group, which will gain `--server`).
    #[arg(long)]
    pub client: bool,
    /// Laboratory id — names the per-state container.
    #[arg(long)]
    pub id: Option<String>,
    /// The EXACT machine id whose host owns the container. Requires
    /// `--machine-state`; neither ⇒ the current machine + the daemon's
    /// own state.
    #[arg(long, requires = "machine_state")]
    pub machine: Option<String>,
    /// The state (on `--machine`) whose host owns the container.
    /// Requires `--machine` — both or neither.
    #[arg(long, requires = "machine")]
    pub machine_state: Option<String>,
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
            crate::cli::command::FromArgsError::path_parse("id", "--id is required".to_string())
        })?;
        if !args.client {
            return Err(crate::cli::command::FromArgsError::path_parse(
                "client",
                "--client is required".to_string(),
            ));
        }
        // Both-or-neither, re-validated beyond clap's mutual
        // `requires` (which only runs for argv-built requests).
        if args.machine.is_some() != args.machine_state.is_some() {
            return Err(crate::cli::command::FromArgsError::path_parse(
                "machine",
                "--machine and --machine-state must be provided together".to_string(),
            ));
        }
        Ok(Self {
            path_type: Path::LaboratoriesDelete,
            kind: Kind::Client,
            id,
            machine: args.machine,
            machine_state: args.machine_state,
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

/// One `/listen` broadcast run of `laboratories delete`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::websocket_listener::UnaryResponse<Response>,
}
