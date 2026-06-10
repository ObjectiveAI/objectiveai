//! `agents logs read pending` — fetch the unread delta for the
//! children spawned by a parent AIH, coalesced into [`ResponseItem`]
//! blocks. Read-and-advance: the per-child watermark
//! (`logs.messages_queue.read_index`) is bumped to the maximum
//! returned id in the same SQL statement, never downgraded.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.read.pending.Request")]
pub struct Request {
    pub path_type: Path,
    pub targets: Vec<Target>,
    /// Skip rows with `logs.messages."index" <= after_id`. Composes
    /// with the per-child watermark (`GREATEST` of the two applies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub after_id: Option<i64>,
    /// Cap on rows scanned per target. Defaults to 1000 server-side
    /// when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub limit: Option<i64>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.read.pending.Path")]
pub enum Path {
    #[serde(rename = "agents/logs/read/pending")]
    AgentsLogsReadPending,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "logs".to_string(),
            "read".to_string(),
            "pending".to_string(),
        ];
        for target in &self.targets {
            argv.push("--target".to_string());
            argv.push(target.into_arg_string());
        }
        if let Some(after_id) = self.after_id {
            argv.push("--after-id".to_string());
            argv.push(after_id.to_string());
        }
        if let Some(limit) = self.limit {
            argv.push("--limit".to_string());
            argv.push(limit.to_string());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

// Share the ResponseItem / part / part-type shapes AND the docker-style
// `Target` parser with `agents logs read all` — same underlying
// `logs.messages` rows surfaced as either the full target slice or
// the watermark-delta slice, same per-target input shape.
pub use super::all::{
    AssistantResponsePart, AssistantResponsePartType, ClientNotificationPart,
    ClientNotificationPartType, ResponseItem, Target, ToolResponsePart, ToolResponsePartType,
};

#[derive(clap::Args)]
pub struct Args {
    /// One or more `--target instance=L[,parent=P]` entries. `parent`
    /// defaults to the cli's own `Config.agent_instance_hierarchy`
    /// when omitted on an individual target. Also accepts
    /// `--target tag=T` and `--target me` (the caller's own AIH).
    #[arg(long = "target", required = true)]
    pub targets: Vec<String>,
    /// Skip rows with `logs.messages."index" <= after_id` per target.
    #[arg(long)]
    pub after_id: Option<i64>,
    /// Cap on rows scanned per target.
    #[arg(long)]
    pub limit: Option<i64>,
    /// jq filter applied to the JSON output.
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
            path_type: Path::AgentsLogsReadPending,
            targets,
            after_id: args.after_id,
            limit: args.limit,
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

pub mod request_schema;


pub mod response_schema;
