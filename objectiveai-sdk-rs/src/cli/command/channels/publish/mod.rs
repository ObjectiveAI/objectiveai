//! `channels publish` — offer a duplex channel and BLOCK until the
//! first connected `/channels` client accepts it (or the base
//! `--timeout` elapses; the wait is uncapped without one). Returns the
//! new `channel_id` and the publisher's per-channel secret (`S_pub`),
//! which authorizes `channels logs request` and publisher-side reads.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.publish.Request")]
pub struct Request {
    pub path_type: Path,
    /// Caller-chosen discriminator (e.g. `"browser.login"`) — how a
    /// user surface decides whether/how to accept the offer.
    pub key: String,
    /// Arbitrary offer payload, opaque to the daemon.
    pub details: serde_json::Value,
    /// Human-readable offer message, opaque to the daemon. Capped at
    /// 512 characters of the raw (unescaped) string.
    pub message: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.publish.Path")]
pub enum Path {
    #[serde(rename = "channels/publish")]
    ChannelsPublish,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The accepted channel: its id and the publisher's per-channel secret
/// (`S_pub`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.publish.Response")]
pub struct Response {
    pub channel_id: String,
    /// The publisher's per-channel capability. Present it to
    /// `channels logs request` / `list` / `open` / `subscribe`.
    pub secret: String,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("key_required").required(true).args(["key"])))]
#[command(group(clap::ArgGroup::new("details_required").required(true).args(["details"])))]
#[command(group(clap::ArgGroup::new("message_required").required(true).args(["message"])))]
pub struct Args {
    /// Caller-chosen discriminator (e.g. "browser.login").
    #[arg(long)]
    pub key: Option<String>,
    /// The offer payload as inline JSON.
    #[arg(long)]
    pub details: Option<String>,
    /// The offer message as an inline JSON string (pre-escaped);
    /// the decoded string is capped at 512 characters.
    #[arg(long)]
    pub message: Option<String>,
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
        let key = args.key.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse("key", "--key is required".to_string())
        })?;
        let details_raw = args.details.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "details",
                "--details is required".to_string(),
            )
        })?;
        let mut de = serde_json::Deserializer::from_str(&details_raw);
        let details = serde_path_to_error::deserialize(&mut de)
            .map_err(|e| crate::cli::command::FromArgsError::json("details", e))?;
        let message_raw = args.message.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "message",
                "--message is required".to_string(),
            )
        })?;
        let mut de = serde_json::Deserializer::from_str(&message_raw);
        let message = serde_path_to_error::deserialize(&mut de)
            .map_err(|e| crate::cli::command::FromArgsError::json("message", e))?;
        Ok(Self {
            path_type: Path::ChannelsPublish,
            key,
            details,
            message,
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

/// One `/listen` broadcast run of `channels publish`. See
/// [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
