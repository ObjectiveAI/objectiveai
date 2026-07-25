//! `tasks list` — every task, one [`ResponseItem`] per stream frame:
//! the stored command + schedule, the run counters (a run is ERRORED
//! iff its last stream item was an error), the last result, and the
//! creator identity the task runs with. Completed tasks stay listed
//! until `tasks delete`.

use crate::cli::command::CommandRequest;

/// The outcome of a task's most recent run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.command.tasks.list.LastResult")]
pub enum LastResult {
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.list.Request")]
pub struct Request {
    pub path_type: Path,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.list.Path")]
pub enum Path {
    #[serde(rename = "tasks/list")]
    TasksList,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// One task.
// No `PartialEq`: the embedded root [`crate::cli::command::Request`]
// doesn't derive it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.list.ResponseItem")]
pub struct ResponseItem {
    /// The task id (`tasks delete --id`).
    pub id: String,
    /// The stored command — the full typed command request. Unboxed:
    /// unlike `create::Request`, this type is not part of the request
    /// enum's own cycle. Schema-opaque (`serde_json::Value`) ON
    /// PURPOSE: embedding the root request enum's schema in a leaf
    /// transitively expands the whole command tree (TS7056) — same
    /// reasoning as `create`. A stored row that no longer parses as
    /// the current request type (a pre-wire-change task) surfaces as
    /// an error ITEM in the list stream rather than a listed entry.
    #[schemars(with = "serde_json::Value")]
    pub command: crate::cli::command::Request,
    /// Seconds until the (first) run / between runs.
    pub delay_secs: u64,
    pub repeat: bool,
    /// Cap on SUCCESSFUL runs — absent = unbounded (or one-shot).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub repeat_count: Option<u64>,
    /// Completed runs (at-least-once: a run lost to a daemon crash
    /// re-fires uncounted).
    pub run_count: u64,
    /// Runs whose LAST stream item was an error.
    pub error_count: u64,
    /// The most recent run's outcome — absent when the task has never
    /// run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub last_result: Option<LastResult>,
    /// The task will not fire again (one-shot that ran, or a counted
    /// repeat whose success budget is met). Stays listed until
    /// deleted.
    pub complete: bool,
    /// RFC3339 creation time.
    pub created_at: String,
    /// RFC3339 next fire time — absent when complete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub next_run_at: Option<String>,
    /// The creator identity the task runs with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_instance_hierarchy: Option<String>,
    /// The creating plugin (owner/repository/version) — present only
    /// when a plugin created the task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_version: Option<String>,
}

#[derive(clap::Args)]
pub struct Args {
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
    /// Emit the JSON Schema for this leaf's `ResponseItem` type and exit.
    ResponseSchema(response_schema::Args),
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        Ok(Self {
            path_type: Path::TasksList,
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
    identity: Option<&crate::identity::Identity>,
) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.base.clear_transform();
    executor.execute(request, identity).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    identity: Option<&crate::identity::Identity>,
) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.base.set_transform(transform);
    executor.execute(request, identity).await
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `tasks list`: the actual
/// [`Request`], the producer's
/// [`Identity`](crate::identity::Identity), and the
/// item stream. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::cli::broadcast_listener::ResponseItemStream<ResponseItem>,
}
