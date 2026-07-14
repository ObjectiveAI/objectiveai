//! `plugins get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.get.Request")]
pub struct Request {
    pub path_type: Path,
    pub owner: String,
    pub name: String,
    pub version: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.get.Path")]
pub enum Path {
    #[serde(rename = "plugins/get")]
    PluginsGet,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

pub type Response = Option<ResponseManifest>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.get.ResponseManifest")]
pub struct ResponseManifest {
    pub owner: String,
    pub name: String,
    pub version: String,
    pub description: String,
    /// Per-OS exec argv for the plugin's cli side, run with CWD =
    /// `<plugin dir>/cli/` — the same shape tools use. Required (an
    /// empty exec is a viewer-only plugin, which still round-trips as
    /// an empty per-OS object).
    pub exec: crate::cli::command::tools::get::Exec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub viewer_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<ResponseMcpServer>,
}

impl ResponseManifest {
    /// LLM-visible MCP tool name for this plugin:
    /// `plugin_{owner}_{name}_{version}`, with every `.` in the
    /// version substituted to `-` so the result stays within the
    /// Anthropic tool-name regex (`^[a-zA-Z0-9_-]{1,128}$`).
    /// `objectiveai-mcp` advertises each plugin under this name.
    pub fn tool_name(&self) -> String {
        format!(
            "plugin_{}_{}_{}",
            self.owner,
            self.name,
            self.version.replace('.', "-")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.get.ResponseMcpServer")]
pub struct ResponseMcpServer {
    pub name: String,
    pub authorization: bool,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("owner_required").required(true).args(["owner"])))]
#[command(group(clap::ArgGroup::new("name_required").required(true).args(["name"])))]
#[command(group(clap::ArgGroup::new("version_required").required(true).args(["version"])))]
pub struct Args {
    /// Plugin owner (GitHub `<owner>` segment). Required.
    #[arg(long)]
    pub owner: Option<String>,
    /// Plugin name (repository segment). Required.
    #[arg(long)]
    pub name: Option<String>,
    /// Plugin version. Required.
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
        Ok(Self {
            path_type: Path::PluginsGet,
            owner: args.owner.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "owner",
                    "--owner is required".to_string(),
                )
            })?,
            name: args.name.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "name",
                    "--name is required".to_string(),
                )
            })?,
            version: args.version.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "version",
                    "--version is required".to_string(),
                )
            })?,
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
impl crate::cli::command::CommandResponse for ResponseManifest {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `plugins get`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
