//! `channels logs open` — reveal one entry's arbitrary-JSON `content`
//! by its `--entry-id`, scoped to the channel. Pure read (never
//! advances a watermark). Authenticate with a channel secret (`S_pub`
//! or `S_owner`).

use crate::cli::command::CommandRequest;

pub use super::list::MessageKind;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.open.Request")]
pub struct Request {
    pub path_type: Path,
    pub channel_id: String,
    pub secret: String,
    /// The `channel_messages.id` to open.
    pub entry_id: i64,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.open.Path")]
pub enum Path {
    #[serde(rename = "channels/logs/open")]
    ChannelsLogsOpen,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The opened entry (envelope + content), or `not_found`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.channels.logs.open.Response")]
pub enum Response {
    #[schemars(title = "Entry")]
    Entry {
        id: i64,
        timestamp: String,
        kind: MessageKind,
        identity: crate::cli::command::AgentArguments,
        content: serde_json::Value,
    },
    #[schemars(title = "NotFound")]
    NotFound,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("channel_required").required(true).args(["id"])))]
#[command(group(clap::ArgGroup::new("secret_required").required(true).args(["secret"])))]
#[command(group(clap::ArgGroup::new("entry_required").required(true).args(["entry_id"])))]
pub struct Args {
    /// The channel id.
    #[arg(long)]
    pub id: Option<String>,
    /// A channel secret (`S_pub` or `S_owner`).
    #[arg(long)]
    pub secret: Option<String>,
    /// The entry id (`channel_messages.id`) to open.
    #[arg(long)]
    pub entry_id: Option<i64>,
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
        let channel_id = args.id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse("id", "--id is required".to_string())
        })?;
        let secret = args.secret.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "secret",
                "--secret is required".to_string(),
            )
        })?;
        let entry_id = args.entry_id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "entry_id",
                "--entry-id is required".to_string(),
            )
        })?;
        Ok(Self {
            path_type: Path::ChannelsLogsOpen,
            channel_id,
            secret,
            entry_id,
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

pub mod request_schema;

pub mod response_schema;

/// See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
