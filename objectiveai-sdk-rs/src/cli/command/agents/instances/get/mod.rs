//! `agents instances get` — fetch per-agent aggregates (tags, queued
//! count, spawn/active timestamps, total logged messages) for one or
//! more targets. Same response shape as `agents instances list`, but
//! each target resolves to the EXACT agent rather than its children;
//! an explicitly-named target always yields an item (zero-filled when
//! it has no activity).

use crate::cli::command::CommandRequest;

/// Reuse the shared `--target` enum and the `list` response item — a
/// `get` row is identical in shape to a `list` row.
pub use super::list::{ResponseItem, Target};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.get.Request")]
pub struct Request {
    pub path_type: Path,
    pub targets: Vec<Target>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.get.Path")]
pub enum Path {
    #[serde(rename = "agents/instances/get")]
    AgentsInstancesGet,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

#[derive(clap::Args)]
pub struct Args {
    /// One or more `--target instance=L[,parent=P]` entries. Also
    /// accepts `--target tag=T` and `--target me`. Fetches each
    /// resolved agent exactly (not its children).
    #[arg(long = "target", required = true)]
    pub targets: Vec<String>,
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
            path_type: Path::AgentsInstancesGet,
            targets,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        identity: Option<&crate::identity::Identity>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.base.clear_transform();
    executor.execute(request, identity).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,

        identity: Option<&crate::identity::Identity>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.base.set_transform(transform);
    executor.execute(request, identity).await
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `agents instances get`: the actual
/// [`Request`], the producer's
/// [`Identity`](crate::identity::Identity), and the
/// response-item stream. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::ResponseItemStream<ResponseItem>,
}
