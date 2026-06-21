//! `plugins daemon notify` — deliver one input to a resident daemon
//! plugin's stdin.
//!
//! Ensures the per-state plugin daemon is up (spawning it if needed),
//! then connects to the target plugin's per-plugin socket and writes
//! `input` as one JSON line (JSONL) to that plugin's stdin. Returns the
//! daemon's ack — success means the input was handed to the plugin's
//! stdin, NOT that the plugin produced any particular output.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.daemon.notify.Request")]
pub struct Request {
    pub path_type: Path,
    pub owner: String,
    pub name: String,
    pub version: String,
    /// The value written to the target plugin's stdin as one JSON line.
    pub input: serde_json::Value,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.daemon.notify.Path")]
pub enum Path {
    #[serde(rename = "plugins/daemon/notify")]
    PluginsDaemonNotify,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.daemon.notify.Response")]
pub struct Response {
    pub ok: bool,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("owner_required").required(true).args(["owner"])))]
#[command(group(clap::ArgGroup::new("name_required").required(true).args(["name"])))]
#[command(group(clap::ArgGroup::new("version_required").required(true).args(["version"])))]
#[command(group(clap::ArgGroup::new("input_required").required(true).args(["input"])))]
pub struct Args {
    /// Plugin owner (GitHub `<owner>` segment).
    #[arg(long)]
    pub owner: Option<String>,
    /// Plugin name (repository segment).
    #[arg(long)]
    pub name: Option<String>,
    /// Plugin version.
    #[arg(long)]
    pub version: Option<String>,
    /// Inline JSON value written to the plugin's stdin as one line.
    #[arg(long)]
    pub input: Option<String>,
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
            crate::cli::command::FromArgsError::path_parse("owner", "--owner is required".to_string())
        })?;
        let name = args.name.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse("name", "--name is required".to_string())
        })?;
        let version = args.version.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse("version", "--version is required".to_string())
        })?;
        let input_str = args.input.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse("input", "--input is required".to_string())
        })?;
        let mut de = serde_json::Deserializer::from_str(&input_str);
        let input = serde_path_to_error::deserialize(&mut de).map_err(|source| {
            crate::cli::command::FromArgsError { field: "input", source: source.into() }
        })?;
        Ok(Self {
            path_type: Path::PluginsDaemonNotify,
            owner,
            name,
            version,
            input,
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
