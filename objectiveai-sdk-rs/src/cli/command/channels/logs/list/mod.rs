//! `channels logs list` — the channel's message log as ENVELOPES
//! (id, timestamp, kind, writer identity); the arbitrary-JSON content
//! is revealed only by `channels logs open`. `--all` lists every
//! entry; `--pending` lists only the entries THIS role hasn't read
//! (messages from the other side past its watermark) and advances the
//! watermark. Authenticate with a channel secret (`S_pub` or
//! `S_owner`) — the daemon infers the role.

use crate::cli::command::CommandRequest;

/// Which side of the channel an entry came from — `request` is
/// publisher→owner, `reply` is owner→publisher. Retained for
/// [`super::open`]'s flat entry; [`ChannelLogEntry`] is now split by
/// this same axis into an enum instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.command.channels.logs.list.MessageKind")]
pub enum MessageKind {
    Request,
    Reply,
}

/// One channel-log entry ENVELOPE — no content (open reveals that).
/// Identity is INLINE and daemon-authored (unspoofable). The two
/// directions are ASYMMETRIC in identity, so the entry is an enum
/// tagged by `kind`:
///
/// - `request` (publisher→owner): the publisher is a PLUGIN — the
///   plugin trio is REQUIRED.
/// - `reply` (owner→publisher): the replier is never a plugin — no
///   plugin trio at all.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[schemars(rename = "cli.command.channels.logs.list.ChannelLogEntry")]
pub enum ChannelLogEntry {
    /// A publisher→owner message. The publisher is a plugin, so the
    /// originating plugin trio is always present.
    #[schemars(title = "Request")]
    Request {
        /// The entry's `channel_messages.id` — the cursor for
        /// `--after-id` and the `--entry-id` for `channels logs open`.
        id: i64,
        /// RFC3339 delivery time.
        timestamp: String,
        /// The AIH of the agent that sent the entry (always present —
        /// the daemon defaults it).
        sender_agent_instance_hierarchy: String,
        /// The originating plugin (owner/repository/version) — always
        /// present: a channel's requester is a plugin.
        plugin_owner: String,
        plugin_repository: String,
        plugin_version: String,
    },
    /// An owner→publisher message. The replier is not a plugin.
    #[schemars(title = "Reply")]
    Reply {
        /// The entry's `channel_messages.id`.
        id: i64,
        /// RFC3339 delivery time.
        timestamp: String,
        /// The AIH of the agent that sent the entry (always present —
        /// the daemon defaults it).
        sender_agent_instance_hierarchy: String,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.list.Request")]
pub struct Request {
    pub path_type: Path,
    pub channel_id: String,
    pub secret: String,
    /// `true` = `--pending` (unread for this role, advances the
    /// watermark); `false` = `--all`.
    pub pending: bool,
    /// Skip entries with `id <= after_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub after_id: Option<i64>,
    /// Cap on entries returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub limit: Option<i64>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.list.Path")]
pub enum Path {
    #[serde(rename = "channels/logs/list")]
    ChannelsLogsList,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The listed entries, ascending by id.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.list.Response")]
pub struct Response {
    pub entries: Vec<ChannelLogEntry>,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("channel_required").required(true).args(["id"])))]
#[command(group(clap::ArgGroup::new("secret_required").required(true).args(["secret"])))]
#[command(group(clap::ArgGroup::new("list_mode").required(true).multiple(false).args(["all", "pending"])))]
pub struct Args {
    /// The channel id.
    #[arg(long)]
    pub id: Option<String>,
    /// A channel secret (`S_pub` or `S_owner`).
    #[arg(long)]
    pub secret: Option<String>,
    /// List every entry.
    #[arg(long)]
    pub all: bool,
    /// List only the entries this role hasn't read (advances the
    /// watermark).
    #[arg(long)]
    pub pending: bool,
    /// Skip entries with id <= after_id.
    #[arg(long)]
    pub after_id: Option<i64>,
    /// Cap on entries returned.
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
        // The ArgGroup guarantees exactly one of --all/--pending.
        Ok(Self {
            path_type: Path::ChannelsLogsList,
            channel_id,
            secret,
            pending: args.pending,
            after_id: args.after_id,
            limit: args.limit,
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
