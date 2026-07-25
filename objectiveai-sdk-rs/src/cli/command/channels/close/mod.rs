//! `channels close` — close a channel (terminal: no further requests
//! or replies; the log survives, readable and listable). EITHER
//! per-channel secret authorizes it — the publisher's (`S_pub`) or
//! the owner's (`S_owner`). Idempotent: closing a closed channel
//! succeeds. Any blocked `channels logs subscribe` unblocks with
//! `channel_closed`.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.close.Request")]
pub struct Request {
    pub path_type: Path,
    /// The channel to close.
    pub channel_id: String,
    /// A channel secret — `S_pub` or `S_owner`; either authorizes.
    pub secret: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.close.Path")]
pub enum Path {
    #[serde(rename = "channels/close")]
    ChannelsClose,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The closed channel's id, echoed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.close.Response")]
pub struct Response {
    pub channel_id: String,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("channel_id_required").required(true).args(["channel_id"])))]
#[command(group(clap::ArgGroup::new("secret_required").required(true).args(["secret"])))]
pub struct Args {
    /// The channel to close.
    #[arg(long)]
    pub channel_id: Option<String>,
    /// A channel secret — S_pub or S_owner; either authorizes.
    #[arg(long)]
    pub secret: Option<String>,
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
        let channel_id = args.channel_id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "channel_id",
                "--channel-id is required".to_string(),
            )
        })?;
        let secret = args.secret.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "secret",
                "--secret is required".to_string(),
            )
        })?;
        Ok(Self {
            path_type: Path::ChannelsClose,
            channel_id,
            secret,
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

/// One `/listen` broadcast run of `channels close`. See
/// [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
