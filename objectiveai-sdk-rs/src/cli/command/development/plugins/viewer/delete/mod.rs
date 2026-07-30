//! `development plugins viewer delete` — drop a viewer development
//! registration, restoring resolution to the installed copy.
//!
//! Nothing to clean up beyond the registration itself: development
//! serving reads files live and caches nothing, which is also why
//! there is no `reset` in this subtier.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.viewer.delete.Request")]
pub struct Request {
    pub path_type: Path,
    /// GitHub `<owner>` segment, lowercased on arrival.
    pub owner: String,
    /// Repository segment, likewise lowercased.
    pub name: String,
    /// Plugin version, matched byte-for-byte.
    pub version: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.viewer.delete.Path")]
pub enum Path {
    #[serde(rename = "development/plugins/viewer/delete")]
    DevelopmentPluginsViewerDelete,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The coordinates, echoed as canonicalized.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.viewer.delete.Response")]
pub struct Response {
    pub owner: String,
    pub name: String,
    pub version: String,
    /// Whether a registration was actually removed. Deleting one that
    /// was never registered is a SUCCESS with `false` — the requested
    /// state ("this plugin is not in development mode") already holds.
    pub removed: bool,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("owner_required").required(true).args(["owner"])))]
#[command(group(clap::ArgGroup::new("name_required").required(true).args(["name"])))]
#[command(group(clap::ArgGroup::new("version_required").required(true).args(["version"])))]
pub struct Args {
    /// GitHub <owner> segment.
    #[arg(long)]
    pub owner: Option<String>,
    /// Repository segment.
    #[arg(long)]
    pub name: Option<String>,
    /// Plugin version, exactly as it was registered.
    #[arg(long)]
    pub version: Option<String>,
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
        let owner = args.owner.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "owner",
                "--owner is required".to_string(),
            )
        })?;
        let name = args.name.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "name",
                "--name is required".to_string(),
            )
        })?;
        let version = args.version.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "version",
                "--version is required".to_string(),
            )
        })?;
        Ok(Self {
            path_type: Path::DevelopmentPluginsViewerDelete,
            owner,
            name,
            version,
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

/// One `/listen` broadcast run of `development plugins viewer delete`.
/// See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
