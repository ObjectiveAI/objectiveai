//! `agents tasks list` — inspection leaf over `schedules`.
//!
//! Scope is one or more `--target` entries (the shared `Target`:
//! `me` / `instance=L[,parent=P]` / `tag=T`). Each resolves to an AIH
//! and the listing returns the schedule rows whose
//! `agent_instance_hierarchy` equals it — exact match, no subtree
//! descent. Only the NEWEST version of each `(name, aih)` lists;
//! `--overwrite`-shadowed versions stay on disk (per-version run
//! history) but never surface here. `--oneshot` / `--interval` filter
//! by kind; `--pending` / `--exhausted` filter by readiness (derived
//! from the schedule's newest `tasks_runs` entry); `--after-id` /
//! `--count` paginate forward by ascending row id.

use crate::cli::command::CommandRequest;

/// The same `Target` every hierarchy-scoped read command uses
/// (`agents instances list`, `agents logs read all`, …).
pub use crate::cli::command::agents::logs::read::all::Target;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.list.Request")]
pub struct Request {
    pub path_type: Path,
    /// One or more targets to list schedules for. Each resolves to a
    /// single AIH (`me` → the cli's own; `instance=L[,parent=P]`;
    /// `tag=T` BOUND-only — PENDING / ABSENT error), and rows whose
    /// `agent_instance_hierarchy` equals any resolved AIH are returned.
    pub targets: Vec<Target>,
    /// Filter to oneshot rows only. Mutually exclusive with `interval`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub oneshot: bool,
    /// Filter to recurring rows only. Mutually exclusive with `oneshot`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub interval: bool,
    /// Show only schedules currently runnable — oneshots that
    /// have never fired, and interval rows whose interval has
    /// elapsed. Mutually exclusive with `exhausted`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pending: bool,
    /// Show only schedules NOT currently runnable — fired
    /// oneshots (visible briefly before the runner deletes
    /// them) and interval rows that are cooling down. Mutually
    /// exclusive with `pending`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exhausted: bool,
    /// Skip rows with `schedules.id <= after_id`. Use the highest
    /// `id` from a previous page to paginate forward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub after_id: Option<i64>,
    /// Per-target row cap — each target's query returns at most this
    /// many rows (ascending id, after `after_id`). `None` = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub count: Option<u64>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.list.Path")]
pub enum Path {
    #[serde(rename = "tasks/list")]
    AgentsTasksList,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "tasks".to_string(),
            "list".to_string(),
        ];
        for target in &self.targets {
            argv.push("--target".to_string());
            argv.push(target.into_arg_string());
        }
        if self.oneshot {
            argv.push("--oneshot".to_string());
        }
        if self.interval {
            argv.push("--interval".to_string());
        }
        if self.pending {
            argv.push("--pending".to_string());
        }
        if self.exhausted {
            argv.push("--exhausted".to_string());
        }
        if let Some(after_id) = self.after_id {
            argv.push("--after-id".to_string());
            argv.push(after_id.to_string());
        }
        if let Some(c) = self.count {
            argv.push("--count".to_string());
            argv.push(c.to_string());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// The plugin that registered a schedule. All three fields are present
/// together or the whole object is absent (the `schedules` table
/// enforces all-or-nothing on its `plugin_*` columns).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.list.Plugin")]
pub struct Plugin {
    pub owner: String,
    pub repository: String,
    pub version: String,
}

/// One schedule row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.list.ResponseItem")]
pub struct ResponseItem {
    /// The `schedules` row id. Monotonic; pass the highest `id` from a
    /// page as the next request's `after_id` to paginate forward.
    pub id: i64,
    /// The `--name` passed to `agents tasks schedule`. Unique per
    /// `agent_instance_hierarchy`.
    pub name: String,
    pub agent_instance_hierarchy: String,
    pub command: Vec<String>,
    pub description: String,
    pub created_at: i64,
    /// Unix seconds of the most recent invocation — this row's newest
    /// `tasks_runs` entry. `None` until the runner has fired this
    /// version at least once (runs are tracked per-version).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub last_ran_at: Option<i64>,
    /// `None` for a oneshot; `Some("30s" / "1h" / "1d12h" / …)`
    /// for a recurring schedule, formatted as humantime so the
    /// list output reads naturally without a unit-conversion
    /// step at the consumer. The CLI parser accepts the same
    /// shape on `agents tasks schedule --interval`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub interval: Option<String>,
    /// This row's version: `1` for a freshly scheduled task,
    /// `max + 1` for each `tasks schedule --overwrite` (each version
    /// is its own row; only the newest lists).
    pub version: u64,
    /// The plugin that registered this schedule (its `(owner,
    /// repository, version)` coordinate), or `None` when it was not
    /// scheduled by a plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin: Option<Plugin>,
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("kind")
        .multiple(false)
        .args(["oneshot", "interval"])
))]
#[command(group(
    clap::ArgGroup::new("readiness")
        .multiple(false)
        .args(["pending", "exhausted"])
))]
pub struct Args {
    /// One or more `--target instance=L[,parent=P]` entries. Also
    /// accepts `--target tag=T` and `--target me`. Lists schedules
    /// whose `agent_instance_hierarchy` equals each resolved AIH.
    #[arg(long = "target", required = true)]
    pub targets: Vec<String>,
    /// Filter to oneshot rows only. Mutually exclusive with `--interval`.
    #[arg(long)]
    pub oneshot: bool,
    /// Filter to recurring rows only. Mutually exclusive with `--oneshot`.
    #[arg(long)]
    pub interval: bool,
    /// Only schedules currently runnable. Mutually exclusive with `--exhausted`.
    #[arg(long)]
    pub pending: bool,
    /// Only schedules NOT currently runnable. Mutually exclusive with `--pending`.
    #[arg(long)]
    pub exhausted: bool,
    /// Skip rows with `schedules.id <= after_id`; use the highest
    /// `id` from the previous page to paginate forward.
    #[arg(long)]
    pub after_id: Option<i64>,
    #[arg(long)]
    pub count: Option<u64>,
    #[arg(long)]
    pub jq: Option<String>,
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
            path_type: Path::AgentsTasksList,
            targets,
            oneshot: args.oneshot,
            interval: args.interval,
            pending: args.pending,
            exhausted: args.exhausted,
            after_id: args.after_id,
            count: args.count,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.jq = None;
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.jq = Some(jq);
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
