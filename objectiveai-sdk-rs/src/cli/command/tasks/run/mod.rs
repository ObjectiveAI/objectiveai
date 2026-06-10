//! `agents tasks run` — fire every pending schedule in the caller's
//! own subtree.
//!
//! Scope is fixed: every schedule whose `agent_instance_hierarchy` is
//! the caller's own AIH or a descendant of it. Of those, the runner
//! fires the pending ones (unfired oneshots + interval rows whose
//! interval has elapsed), bumps each fired row's `last_ran_at` to
//! `now`, deletes the oneshots it fired, and dispatches each row's
//! stored argv through the root `crate::run` in parallel — with the
//! schedule's captured identity (and the plugin that registered it, if
//! any) re-installed on the run ctx. The per-task streams are merged;
//! each emitted item is wrapped with the source schedule's `id`.

use crate::cli::command::CommandRequest;

/// The plugin that registered a schedule — the same shape `tasks list`
/// surfaces.
pub use crate::cli::command::tasks::list::Plugin;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.run.Request")]
pub struct Request {
    pub path_type: Path,
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
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// One output item from one fired schedule's in-process stream. The
/// first four fields identify the source schedule; `value` is the
/// typed root [`crate::cli::command::ResponseItem`] emitted by the
/// scheduled cli leaf — boxed because the root union transitively
/// contains *this* variant (`agents → tasks → run`), and boxing is
/// what makes the recursion sized.
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
    /// The source schedule's `schedules` row id.
    pub id: i64,
    /// The source schedule's `agent_instance_hierarchy`.
    pub agent_instance_hierarchy: String,
    /// The source schedule's `--name`.
    pub name: String,
    /// The plugin that registered the source schedule, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin: Option<Plugin>,
    /// The typed root item emitted by the scheduled command.
    #[schemars(with = "serde_json::Value")]
    pub value: Box<crate::cli::command::ResponseItem>,
}

#[derive(clap::Args)]
pub struct Args {
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
