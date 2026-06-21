//! `agents queue read pending` — stream every pending (i.e.
//! `active = TRUE`) `message_queue` row for the resolved targets,
//! coalesced into parts-grouped `ResponseItem` blocks. Symmetric
//! with `agents logs read all`'s `ClientNotification` shape — same
//! `Target` input, same per-part type tag, same `id` shape (an i64
//! you pass to the matching `read id` verb to drill into the body).

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.list.Request")]
pub struct Request {
    pub path_type: Path,
    pub targets: Vec<Target>,
    /// Skip rows with `message_queue_contents.id <= after_id`. Use
    /// the highest `id` from a previous page to paginate forward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub after_id: Option<i64>,
    /// Cap on rows scanned per target. `None` = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub limit: Option<i64>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.list.Path")]
pub enum Path {
    #[serde(rename = "agents/queue/list")]
    AgentsQueueList,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

// Share `Target` + `ClientNotificationPartType` with
// `agents logs read all` — same per-target input shape, same
// per-part 5-variant kind discriminator. `agents queue read pending`
// is the pre-execution mirror of logs read all's
// `ClientNotification` block.
pub use super::super::logs::read::all::{
    ClientNotificationPartType as QueuePartType, Target,
};

/// One row inside a `ResponseItem` block — a
/// `message_queue_contents` entry. The `id` is the
/// `message_queue_contents.id`, which you pass to
/// `agents queue read id <n>` to drill into the body.
/// `enqueued_at` is on the enclosing block, not here
/// (one block = one `message_queue` parent row, sharing one
/// `enqueued_at`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.list.QueuePart")]
pub struct QueuePart {
    pub id: i64,
    pub r#type: QueuePartType,
}

/// One pending `message_queue` row, with its content rows
/// grouped as `parts`. Two variants — direct AIH target or tag
/// target — both flat (no nested `LookupState`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.queue.list.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "AgentInstanceHierarchy")]
    AgentInstanceHierarchy {
        /// `message_queue.id` — the row-level id this block
        /// represents. Pass to `agents queue delete <id>` to
        /// soft-flip the entire row (all parts) in one call.
        /// Distinct from each `QueuePart.id` (which is a
        /// `message_queue_contents.id` for drilling into one
        /// content slot via `agents queue read id`).
        delete_id: i64,
        agent_instance_hierarchy: String,
        /// AIH of the caller who enqueued — from
        /// `message_queue.sender_*`.
        sender_agent_instance_hierarchy: String,
        /// `message_queue.enqueued_at`. One block = one parent
        /// `message_queue` row, so this is well-defined
        /// block-level.
        enqueued_at: String,
        /// Idempotency token, if the row was enqueued with `--key`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        key: Option<String>,
        parts: Vec<QueuePart>,
    },
    #[schemars(title = "Tag")]
    Tag {
        /// `message_queue.id`. Pass to `agents queue delete <id>`.
        delete_id: i64,
        agent_tag: String,
        sender_agent_instance_hierarchy: String,
        enqueued_at: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        key: Option<String>,
        parts: Vec<QueuePart>,
    },
}

#[derive(clap::Args)]
pub struct Args {
    /// List pending (undelivered) queued prompts. Required: today
    /// `list` only supports the pending view, so the flag must be
    /// given explicitly (leaves room for future list modes).
    #[arg(long, required = true)]
    pub pending: bool,
    /// One or more `--target instance=L[,parent=P]` entries.
    /// `parent` defaults to the cli's own
    /// `Config.agent_instance_hierarchy` when omitted on an
    /// individual target. Also accepts `--target tag=T` and
    /// `--target me` (the caller's own AIH).
    #[arg(long = "target", required = true)]
    pub targets: Vec<String>,
    /// Skip rows with `message_queue_contents.id <= after_id`.
    #[arg(long)]
    pub after_id: Option<i64>,
    /// Cap on rows scanned per target.
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
        let targets = args
            .targets
            .iter()
            .map(|s| {
                s.parse::<Target>().map_err(|msg| {
                    crate::cli::command::FromArgsError::path_parse("target", msg)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            path_type: Path::AgentsQueueList,
            targets,
            after_id: args.after_id,
            limit: args.limit,
            base: args.base.into(),
        })
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

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;
