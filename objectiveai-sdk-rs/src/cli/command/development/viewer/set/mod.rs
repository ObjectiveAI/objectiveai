//! `development viewer set` — run the viewer FROM SOURCE.
//!
//! Registers the `objectiveai-viewer` source directory; from then on
//! `viewer spawn` runs `pnpm exec tauri dev` in it instead of the
//! installed binary. A RUNNING viewer is killed and relaunched in the
//! new form immediately; an absent one is not launched, but every
//! future spawn uses the registration. A source build that fails to
//! start FAILS THIS COMMAND, with the build output in the error.
//!
//! In-memory like every development registration: gone on daemon
//! restart, by design.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.viewer.set.Request")]
pub struct Request {
    pub path_type: Path,
    /// ABSOLUTE path to the `objectiveai-viewer` directory of a source
    /// checkout (the one holding `tauri.conf.json`; pnpm resolves the
    /// workspace upward from it).
    pub path: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.viewer.set.Path")]
pub enum Path {
    #[serde(rename = "development/viewer/set")]
    DevelopmentViewerSet,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The registration, echoed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.viewer.set.Response")]
pub struct Response {
    pub path: String,
    /// Whether this replaced a previous registration.
    pub replaced: bool,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("path_required").required(true).args(["path"])))]
pub struct Args {
    /// Absolute path to the objectiveai-viewer source directory.
    #[arg(long)]
    pub path: Option<String>,
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        let path = args.path.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "path",
                "--path is required".to_string(),
            )
        })?;
        Ok(Self {
            path_type: Path::DevelopmentViewerSet,
            path,
            base: args.base.into(),
        })
    }
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

/// One `/listen` broadcast run of `development viewer set`. See
/// [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
