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
pub struct Args {
    /// Caller-chosen discriminator (e.g. "browser.login").
    #[arg(long)]
    pub key: Option<String>,
    /// The offer payload as inline JSON.
    #[arg(long)]
    pub details: Option<String>,
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
        Ok(Self {
            path_type: Path::ChannelsPublish,
            key,
            details,
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

/// One `/listen` broadcast run of `channels publish`. See
/// [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
