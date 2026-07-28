//! `development plugins mcp create` — point a plugin's coordinates at
//! a local directory, so the laboratory host builds its image from
//! that tree instead of fetching the version's git tag.
//!
//! The registration lives IN THE RESIDENT DAEMON's memory and nowhere
//! else — no database, no file. It is a developer's session state, and
//! surviving a daemon restart would mean a stale registration silently
//! outliving the work it was for.
//!
//! Registering also pins the plugin to the LOCAL laboratory host: the
//! directory is a path on this machine, so no other host could build
//! from it.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.mcp.create.Request")]
pub struct Request {
    pub path_type: Path,
    /// GitHub `<owner>` segment, lowercased on arrival — the
    /// declaration layer lowercases it too, so the registration and an
    /// agent's `plugin` entry agree.
    pub owner: String,
    /// Repository segment, likewise lowercased.
    pub name: String,
    /// Plugin version — matched byte-for-byte against what an agent
    /// declares. It is the git tag everywhere else, and stays
    /// case-sensitive here even though nothing is fetched.
    pub version: String,
    /// ABSOLUTE path to the plugin's source directory: the one holding
    /// `objectiveai.json`. Absolute because the laboratory host
    /// resolves it, and its working directory is not yours.
    pub path: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.mcp.create.Path")]
pub enum Path {
    #[serde(rename = "development/plugins/mcp/create")]
    DevelopmentPluginsMcpCreate,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The registration, echoed with the coordinates as the daemon
/// canonicalized them — which is what an agent's declaration will be
/// matched against, so it is worth seeing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.mcp.create.Response")]
pub struct Response {
    pub owner: String,
    pub name: String,
    pub version: String,
    pub path: String,
    /// Whether this REPLACED an existing registration for the same
    /// coordinates. Worth reporting: re-registering a different
    /// directory is silent otherwise, and the image built from the old
    /// one is still tagged until the next create rebuilds it.
    pub replaced: bool,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("owner_required").required(true).args(["owner"])))]
#[command(group(clap::ArgGroup::new("name_required").required(true).args(["name"])))]
#[command(group(clap::ArgGroup::new("version_required").required(true).args(["version"])))]
#[command(group(clap::ArgGroup::new("path_required").required(true).args(["path"])))]
pub struct Args {
    /// GitHub <owner> segment.
    #[arg(long)]
    pub owner: Option<String>,
    /// Repository segment.
    #[arg(long)]
    pub name: Option<String>,
    /// Plugin version, exactly as an agent declares it (v-prefixed).
    #[arg(long)]
    pub version: Option<String>,
    /// Absolute path to the directory holding objectiveai.json.
    #[arg(long)]
    pub path: Option<String>,
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
        let path = args.path.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "path",
                "--path is required".to_string(),
            )
        })?;
        Ok(Self {
            path_type: Path::DevelopmentPluginsMcpCreate,
            owner,
            name,
            version,
            path,
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

/// One `/listen` broadcast run of `development plugins mcp create`.
/// See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
