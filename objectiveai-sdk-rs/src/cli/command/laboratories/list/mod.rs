//! `laboratories list` — stream the laboratory containers created in this
//! state (podman containers the conduit dials as client-side MCP servers).
//! Read-only; reads back from podman (the source of truth). `--client` is
//! required (a required arg-group, leaving room to add `--server` later).
//! Each item echoes a laboratory's spec: `{ id, image, mounts, env, cwd }`.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.list.Request")]
pub struct Request {
    pub path_type: Path,
    pub kind: super::create::Kind,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.list.Path")]
pub enum Path {
    #[serde(rename = "laboratories/list")]
    LaboratoriesList,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// One laboratory served by a connected laboratory HOST. There is no
/// local-vs-remote split — machine identity is the only provenance,
/// the same logic regardless of where the host runs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.list.ResponseItem")]
pub struct ResponseItem {
    pub id: String,
    pub image: String,
    pub mounts: Vec<super::create::Mount>,
    pub env: Vec<super::create::EnvVar>,
    pub cwd: String,
    /// Unix seconds when the laboratory container was created, from
    /// podman's container record. `None` when the host didn't report
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<i64>,
    /// The machine whose laboratory host serves this laboratory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<crate::machine::MachineIdentity>,
    /// The state (on that machine) the serving host serves —
    /// laboratory ids are only unique per (machine, state).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("side").required(true).args(["client"])))]
pub struct Args {
    /// List client-side laboratories (MCP servers the conduit dials).
    /// Required (part of the `side` arg-group, which will gain `--server`).
    #[arg(long)]
    pub client: bool,
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
        if !args.client {
            return Err(crate::cli::command::FromArgsError::path_parse(
                "client",
                "--client is required".to_string(),
            ));
        }
        Ok(Self {
            path_type: Path::LaboratoriesList,
            kind: super::create::Kind::Client,
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

/// One `/listen` broadcast run of `laboratories list`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// response-item stream. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::websocket_listener::ResponseItemStream<ResponseItem>,
}
