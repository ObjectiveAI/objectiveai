//! `agents tasks run` — fire every pending schedule in scope.
//!
//! Walks every row that `agents tasks list --pending` would
//! surface under the given scope, bumps each row's `last_ran_at`
//! to `now`, deletes oneshots, and dispatches each row's stored
//! argv through the in-process `CliCommandExecutor` in parallel.
//! The resulting per-task streams are merged via SelectAll; each
//! emitted item is wrapped with the source schedule's `name` so
//! callers can attribute output.
//!
//! Only the three scope flags from #216's runner spec —
//! `--agent-instance-hierarchy` / `--tag` (mutex, neither
//! required) and `--depth`. Readiness filtering is implicit: the
//! runner only fires pending rows by definition.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.run.Request")]
pub struct Request {
    pub path_type: Path,
    /// Literal hierarchy scope. Mutually exclusive with `tag`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_instance_hierarchy: Option<String>,
    /// Tag name; resolved BOUND-only at handler time. PENDING /
    /// ABSENT raise structured errors. Mutually exclusive with
    /// `agent_instance_hierarchy`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tag: Option<String>,
    /// Cap descent depth from the scope root. `None` = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub depth: Option<u64>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.run.Path")]
pub enum Path {
    #[serde(rename = "tasks/run")]
    AgentsTasksRun,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "tasks".to_string(),
            "run".to_string(),
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
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// One output item from one fired schedule's in-process stream.
/// `id` is the source schedule's stable identifier, formatted
/// `"{name}-{db_id}"` so it's both human-readable (the user-
/// supplied `--name` from `schedule`) and globally unique (the
/// row id from `schedules`). `value` is the typed root
/// [`crate::cli::command::ResponseItem`] emitted by the scheduled
/// cli leaf — boxed because the root union transitively contains
/// *this* variant (`agents → tasks → run`), and boxing is what
/// makes the recursion sized.
///
/// The `value` field's JSON schema is opaqued to `serde_json::Value`
/// (renders as bare `{}` aka JsonValue) so the published schema
/// doesn't inline the entire root union — that's the TS7056 blowup
/// the root and tier aggregates dodge by being `json_schema_ignore`.
/// Downstream SDKs see `value: JsonValue` on the typed `execute`
/// path; consumers that want to peer inside parse it case-by-case.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.run.ResponseItem")]
pub struct ResponseItem {
    pub id: String,
    #[schemars(with = "serde_json::Value")]
    pub value: Box<crate::cli::command::ResponseItem>,
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("scope")
        .multiple(false)
        .args(["agent_instance_hierarchy", "tag"])
))]
pub struct Args {
    /// Subtree root for the hierarchy filter. Mutually exclusive
    /// with `--tag`. When neither is set, the cli's own
    /// `Config.agent_instance_hierarchy` is used.
    #[arg(long)]
    pub agent_instance_hierarchy: Option<String>,
    /// Tag name; resolved against `tags.sqlite` BOUND-only.
    /// Mutually exclusive with `--agent-instance-hierarchy`.
    #[arg(long)]
    pub tag: Option<String>,
    /// Cap the descent depth from the scope root. Omit for
    /// unlimited descent.
    #[arg(long)]
    pub depth: Option<u64>,
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
            path_type: Path::AgentsTasksRun,
            agent_instance_hierarchy: args.agent_instance_hierarchy,
            tag: args.tag,
            depth: args.depth,
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
