//! `agents tasks list` — inspection leaf over `schedules`.
//!
//! Every filter is optional and additive. Hierarchy scope is
//! always `parent + descendants` — either `--agent-instance-hierarchy
//! <h>` (literal), `--tag <name>` (resolves through `tags.sqlite`
//! BOUND only — errors on PENDING / ABSENT), or, when neither is
//! given, the cli's own `Config.agent_instance_hierarchy`.
//! `--depth N` caps the descent depth; `--oneshot` / `--interval`
//! filter by kind; `--pending` / `--exhausted` filter by
//! readiness. `--offset` / `--count` paginate.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.list.Request")]
pub struct Request {
    pub path_type: Path,
    /// Subtree root for the hierarchy filter. When omitted (and
    /// `tag` is also `None`), the handler substitutes the cli's
    /// own `Config.agent_instance_hierarchy`. Mutually exclusive
    /// with `tag`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_instance_hierarchy: Option<String>,
    /// Tag name; resolved against `tags.sqlite` BOUND-only at
    /// handler time. PENDING / ABSENT lookups return an error.
    /// Mutually exclusive with `agent_instance_hierarchy`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tag: Option<String>,
    /// Maximum descent depth from the hierarchy root. `0` = the
    /// hierarchy itself only; `1` = direct children; `None` =
    /// unlimited recursion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub depth: Option<u64>,
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
    /// Row offset for pagination. Defaults to 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub offset: Option<u64>,
    /// Row count limit for pagination. `None` = unlimited.
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
        if let Some(h) = &self.agent_instance_hierarchy {
            argv.push("--agent-instance-hierarchy".to_string());
            argv.push(h.clone());
        }
        if let Some(t) = &self.tag {
            argv.push("--tag".to_string());
            argv.push(t.clone());
        }
        if let Some(d) = self.depth {
            argv.push("--depth".to_string());
            argv.push(d.to_string());
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
        if let Some(o) = self.offset {
            argv.push("--offset".to_string());
            argv.push(o.to_string());
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

/// One schedule row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.list.ResponseItem")]
pub struct ResponseItem {
    /// Stable identifier — `"{name}-{db_id}"` where `name` is the
    /// `--name` passed to `agents tasks schedule` and `db_id` is
    /// the row id from `schedules`. Same shape `agents tasks run`
    /// tags each emitted item with.
    pub id: String,
    pub agent_instance_hierarchy: String,
    pub command: Vec<String>,
    pub description: String,
    pub created_at: i64,
    /// Unix seconds of the most recent invocation. `None` until
    /// the runner has fired this schedule at least once.
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
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("scope")
        .multiple(false)
        .args(["agent_instance_hierarchy", "tag"])
))]
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
    /// Filter rows to this subtree (inclusive of the parent).
    /// Mutually exclusive with `--tag`. When neither is given,
    /// the cli's own `Config.agent_instance_hierarchy` is used.
    #[arg(long)]
    pub agent_instance_hierarchy: Option<String>,
    /// Tag name; resolved against `tags.sqlite` (BOUND-only —
    /// PENDING / ABSENT raise structured errors). Mutually
    /// exclusive with `--agent-instance-hierarchy`.
    #[arg(long)]
    pub tag: Option<String>,
    /// Cap the descent depth from the hierarchy root.
    /// `0` = root only; `1` = root + direct children. Omit for
    /// unlimited descent.
    #[arg(long)]
    pub depth: Option<u64>,
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
    #[arg(long)]
    pub offset: Option<u64>,
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
        Ok(Self {
            path_type: Path::AgentsTasksList,
            agent_instance_hierarchy: args.agent_instance_hierarchy,
            tag: args.tag,
            depth: args.depth,
            oneshot: args.oneshot,
            interval: args.interval,
            pending: args.pending,
            exhausted: args.exhausted,
            offset: args.offset,
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
