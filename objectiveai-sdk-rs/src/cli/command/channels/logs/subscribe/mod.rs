//! `channels logs subscribe` — the live cousin of `list --pending`.
//! Returns IMMEDIATELY if unread entries already exist; otherwise
//! blocks until a new matching entry arrives or the channel closes,
//! then returns. Yields envelope [`ChannelLogEntry`] items (advancing
//! the role's watermark), or the terminal `"channel_closed"` when the
//! channel is closed with nothing pending. Authenticate with a channel
//! secret (`S_pub` or `S_owner`).

use crate::cli::command::CommandRequest;

pub use super::list::ChannelLogEntry;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.subscribe.Request")]
pub struct Request {
    pub path_type: Path,
    pub channel_id: String,
    pub secret: String,
    /// Skip entries with `id <= after_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub after_id: Option<i64>,
    /// Cap on entries returned per wake.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub limit: Option<i64>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.subscribe.Path")]
pub enum Path {
    #[serde(rename = "channels/logs/subscribe")]
    ChannelsLogsSubscribe,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Subscribe's wire shape. Either a real envelope entry (the EXACT
/// same shape `list` emits) OR the literal string `"channel_closed"`.
/// `#[serde(untagged)]` so the `Item` arm passes through transparently.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.channels.logs.subscribe.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Item")]
    Item(ChannelLogEntry),
    #[schemars(title = "ChannelClosed")]
    ChannelClosed(ChannelClosedTag),
}

/// Single-variant enum whose lone variant serializes as the literal
/// string `"channel_closed"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.subscribe.ChannelClosedTag")]
pub enum ChannelClosedTag {
    #[serde(rename = "channel_closed")]
    ChannelClosed,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("channel_required").required(true).args(["id"])))]
#[command(group(clap::ArgGroup::new("secret_required").required(true).args(["secret"])))]
pub struct Args {
    /// The channel id.
    #[arg(long)]
    pub id: Option<String>,
    /// A channel secret (`S_pub` or `S_owner`).
    #[arg(long)]
    pub secret: Option<String>,
    /// Skip entries with id <= after_id.
    #[arg(long)]
    pub after_id: Option<i64>,
    /// Cap on entries returned per wake.
    #[arg(long)]
    pub limit: Option<i64>,
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
        Ok(Self {
            path_type: Path::ChannelsLogsSubscribe,
            channel_id,
            secret,
            after_id: args.after_id,
            limit: args.limit,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
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

pub mod request_schema;

pub mod response_schema;

/// See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::ResponseItemStream<ResponseItem>,
}
