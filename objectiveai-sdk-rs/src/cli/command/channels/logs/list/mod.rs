//! `channels logs list` — the channel's message log as ENVELOPES
//! (id, timestamp, kind, writer identity); the arbitrary-JSON content
//! is revealed only by `channels logs open`. `--all` lists every
//! entry; `--pending` lists only the entries THIS role hasn't read
//! (messages from the other side past its watermark) and advances the
//! watermark. Authenticate with a channel secret (`S_pub` or
//! `S_owner`) — the daemon infers the role.

use crate::cli::command::CommandRequest;

/// What a stored entry IS — `request` is publisher→owner, `reply` is
/// owner→publisher, and the two `publish*` kinds are the accept-time
/// SEED rows holding the offer (`publish` = the details,
/// `publish_message` = the human message; the pair surfaces in
/// [`ChannelLogEntry`] as ONE `publish` item). Retained for
/// [`super::open`]'s flat entry; [`ChannelLogEntry`] is split into an
/// enum along this same axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.command.channels.logs.list.MessageKind")]
pub enum MessageKind {
    Request,
    Reply,
    Publish,
    PublishMessage,
}

/// One channel-log entry ENVELOPE — no content (open reveals that).
/// Identity is INLINE and daemon-authored (unspoofable). The kinds
/// are ASYMMETRIC in identity and shape, so the entry is an enum
/// tagged by `kind`:
///
/// - `publish` (the accept-time seed, every channel's FIRST entry):
///   the offer itself — `details_id` opens the offer details,
///   `message_id` opens the human message. Publisher-authored, so
///   the plugin trio is REQUIRED.
/// - `request` (publisher→owner): the publisher is a PLUGIN — the
///   plugin trio is REQUIRED.
/// - `reply` (owner→publisher): the replier is never a plugin — no
///   plugin trio at all.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[schemars(rename = "cli.command.channels.logs.list.ChannelLogEntry")]
pub enum ChannelLogEntry {
    /// The channel's offer, seeded at accept — ONE per channel, its
    /// first entry. Two openable ids so a reader pulls the bulky
    /// parts only when it wants them.
    #[schemars(title = "Publish")]
    Publish {
        /// The DETAILS entry id — `channels logs open` reveals the
        /// offer's `details` JSON. Also the `--after-id` cursor.
        details_id: i64,
        /// The MESSAGE entry id — `channels logs open` reveals the
        /// offer's human-readable message.
        message_id: i64,
        /// RFC3339 delivery time (the channel's accept time).
        timestamp: String,
        /// The AIH of the publishing agent (always present — the
        /// daemon defaults it).
        sender_agent_instance_hierarchy: String,
        /// The originating plugin (owner/repository/version) — always
        /// present: a channel's publisher is a plugin.
        plugin_owner: String,
        plugin_name: String,
        plugin_version: String,
    },
    /// A publisher→owner message. The publisher is a plugin, so the
    /// originating plugin trio is always present.
    #[schemars(title = "Request")]
    Request {
        /// The entry's `channel_messages.id` — the cursor for
        /// `--after-id` and the `--entry-id` for `channels logs open`
        /// (revealing the entry's content).
        details_id: i64,
        /// RFC3339 delivery time.
        timestamp: String,
        /// The AIH of the agent that sent the entry (always present —
        /// the daemon defaults it).
        sender_agent_instance_hierarchy: String,
        /// The originating plugin (owner/repository/version) — always
        /// present: a channel's requester is a plugin.
        plugin_owner: String,
        plugin_name: String,
        plugin_version: String,
    },
    /// An owner→publisher message. The replier is typically not a
    /// plugin, so the plugin trio is OPTIONAL — present only when a
    /// plugin happened to send the reply.
    #[schemars(title = "Reply")]
    Reply {
        /// The entry's `channel_messages.id` — the cursor for
        /// `--after-id` and the `--entry-id` for `channels logs open`
        /// (revealing the entry's content).
        details_id: i64,
        /// RFC3339 delivery time.
        timestamp: String,
        /// The AIH of the agent that sent the entry (always present —
        /// the daemon defaults it).
        sender_agent_instance_hierarchy: String,
        /// The originating plugin (owner/repository/version) — present
        /// only when a plugin sent the reply.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        plugin_owner: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        plugin_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        plugin_version: Option<String>,
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
