//! `channels logs reply` — the OWNER→publisher write. Append a `reply`
//! entry (arbitrary-JSON `content`) to the channel log. Requires the
//! owner secret (`S_owner`, received on accept) and an OPEN channel.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.reply.Request")]
pub struct Request {
    pub path_type: Path,
    pub channel_id: String,
    /// The owner secret (`S_owner`).
    pub secret: String,
    /// Arbitrary message payload, opaque to the daemon.
    pub content: serde_json::Value,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.reply.Path")]
pub enum Path {
    #[serde(rename = "channels/logs/reply")]
    ChannelsLogsReply,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The appended entry's id + delivery time. `channel_closed` when the
/// channel is closed — the write was refused.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.channels.logs.reply.Response")]
pub enum Response {
    #[schemars(title = "Appended")]
    Appended { id: i64, timestamp: String },
    #[schemars(title = "ChannelClosed")]
    ChannelClosed,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("channel_required").required(true).args(["id"])))]
#[command(group(clap::ArgGroup::new("secret_required").required(true).args(["secret"])))]
#[command(group(clap::ArgGroup::new("content_required").required(true).args(["content"])))]
pub struct Args {
    /// The channel id.
    #[arg(long)]
    pub id: Option<String>,
    /// The owner secret (`S_owner`).
    #[arg(long)]
    pub secret: Option<String>,
    /// The message payload as inline JSON.
    #[arg(long)]
    pub content: Option<String>,
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
        let content_raw = args.content.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "content",
                "--content is required".to_string(),
            )
        })?;
        let mut de = serde_json::Deserializer::from_str(&content_raw);
        let content = serde_path_to_error::deserialize(&mut de)
            .map_err(|e| crate::cli::command::FromArgsError::json("content", e))?;
        Ok(Self {
            path_type: Path::ChannelsLogsReply,
            channel_id,
            secret,
            content,
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

/// See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
