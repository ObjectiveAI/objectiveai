//! `agents instances list` — enumerate agent instances under one or
//! more targets, reporting per-agent aggregates (tags, queued count,
//! spawn/active timestamps, total logged messages).

use crate::cli::command::CommandRequest;

/// Reuse the shared `--target` enum (`instance=L[,parent=P]`, `tag=T`,
/// `me`) from `agents logs read all`. Each target resolves to an AIH
/// whose descendants this command lists.
pub use super::super::logs::read::all::Target;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.list.Request")]
pub struct Request {
    pub path_type: Path,
    pub targets: Vec<Target>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.list.Path")]
pub enum Path {
    #[serde(rename = "agents/instances/list")]
    AgentsInstancesList,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "instances".to_string(),
            "list".to_string(),
        ];
        for target in &self.targets {
            argv.push("--target".to_string());
            argv.push(target.into_arg_string());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// One discovered agent instance under a target. Aggregated from the
/// `logs.messages`, `message_queue`, and `tags` tiers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.list.ResponseItem")]
pub struct ResponseItem {
    /// Full hierarchy of this agent instance.
    pub agent_instance_hierarchy: String,
    /// Tag names currently bound to this AIH, newest-bound first.
    pub tags: Vec<String>,
    /// Active `message_queue` rows targeting this agent — counting
    /// both direct-AIH rows and rows whose tag is bound to this AIH.
    pub queued: u64,
    /// Timestamp of the first `logs.messages` row for this agent.
    /// `None` when the agent has no logs yet (queue-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub timestamp_spawned: Option<i64>,
    /// Timestamp of the most recent `logs.messages` row for this
    /// agent. `None` when the agent has no logs yet (queue-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub timestamp_active: Option<i64>,
    /// Total `logs.messages` rows for this agent over all time.
    pub logged: u64,
}

#[derive(clap::Args)]
pub struct Args {
    /// One or more `--target instance=L[,parent=P]` entries. Also
    /// accepts `--target tag=T` and `--target me`. Lists every agent
    /// whose AIH is a descendant of each resolved target.
    #[arg(long = "target", required = true)]
    pub targets: Vec<String>,
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
            path_type: Path::AgentsInstancesList,
            targets,
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
