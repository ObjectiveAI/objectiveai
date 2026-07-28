//! `development plugins mcp reset` — drop the plugin's built image so
//! the next run rebuilds it.
//!
//! This is the per-edit verb. A registered plugin still takes the
//! image-exists fast path, so an edit does not take effect on its own;
//! rebuilds are explicit, and this is what asks for one.
//!
//! Forwarded to the LOCAL laboratory host, which holds the image. It
//! does not require a registration — it names an image tag, so it is
//! equally a way to force a released plugin to be re-fetched.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.mcp.reset.Request")]
pub struct Request {
    pub path_type: Path,
    /// GitHub `<owner>` segment, lowercased on arrival.
    pub owner: String,
    /// Repository segment, likewise lowercased.
    pub name: String,
    /// Plugin version — with owner and name, it derives the image tag.
    pub version: String,
    /// Also delete the plugin's build CACHE directories.
    ///
    /// Off by default, and that default is the point: this command
    /// runs after every edit, and the caches are the only reason the
    /// rebuild it triggers is fast. Ask for this when a cache is
    /// corrupt or a toolchain changed, not routinely.
    #[serde(default)]
    pub caches: bool,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.mcp.reset.Path")]
pub enum Path {
    #[serde(rename = "development/plugins/mcp/reset")]
    DevelopmentPluginsMcpReset,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// What the host actually dropped.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.mcp.reset.Response")]
pub struct Response {
    pub owner: String,
    pub name: String,
    pub version: String,
    /// Whether an image was removed. `false` for a plugin that was
    /// never built is a SUCCESS — the point of the command is that the
    /// next run rebuilds, and that is already true.
    pub removed: bool,
    /// Cache directories deleted; always 0 without `--caches`.
    pub caches_removed: u64,
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
    /// Plugin version.
    #[arg(long)]
    pub version: Option<String>,
    /// Also delete the build caches. Slower next build; use only when
    /// a cache is suspect.
    #[arg(long)]
    pub caches: bool,
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
            path_type: Path::DevelopmentPluginsMcpReset,
            owner,
            name,
            version,
            caches: args.caches,
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

/// One `/listen` broadcast run of `development plugins mcp reset`. See
/// [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
