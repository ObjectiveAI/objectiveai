//! `agents tags apply` — bind a tag to an agent instance hierarchy.
//!
//! Three target shapes, mutually exclusive via the `apply_by` clap
//! group:
//!
//! - `--me` — BOUND immediately to the cli's own
//!   `Config.agent_instance_hierarchy`. Forbids
//!   `--parent-agent-instance-hierarchy`.
//! - `--agent-full-id <id>` — PENDING. Recorded against the
//!   `(agent_full_id, parent_agent_instance_hierarchy)` pair; the
//!   next agent-completion spawn whose first chunk reports a matching
//!   pair auto-promotes the tag to BOUND. Optional
//!   `--parent-agent-instance-hierarchy` defaults to the cli's own
//!   `Config.agent_instance_hierarchy` (matching `agents message`).
//! - `--agent-instance <inst>` — BOUND immediately to
//!   `{parent}/{instance}`. Optional
//!   `--parent-agent-instance-hierarchy` defaults to ctx own. Rootless
//!   parents (empty string) yield just `{instance}`, matching the
//!   PENDING → BOUND promotion in `tags::upgrade`.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.apply.Request")]
pub struct Request {
    pub path_type: Path,
    pub name: String,
    pub target: Target,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub jq: Option<String>,
}

/// Apply target. `Me` binds the tag immediately to the cli's own
/// hierarchy. `AgentFullId` records a PENDING row (the existing
/// auto-promotion flow). `AgentInstance` binds immediately to
/// `{parent}/{instance}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.tags.apply.Target")]
pub enum Target {
    #[schemars(title = "Me")]
    Me,
    #[schemars(title = "AgentFullId")]
    AgentFullId {
        agent_full_id: String,
        /// Optional parent scope. `None` ⇒ cli substitutes
        /// `Config.agent_instance_hierarchy`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
    },
    #[schemars(title = "AgentInstance")]
    AgentInstance {
        agent_instance: String,
        /// Optional parent scope. `None` ⇒ cli substitutes
        /// `Config.agent_instance_hierarchy`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.apply.Path")]
pub enum Path {
    #[serde(rename = "agents/tags/apply")]
    AgentsTagsApply,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "tags".to_string(),
            "apply".to_string(),
            "--name".to_string(),
            self.name.clone(),
        ];
        match &self.target {
            Target::Me => {
                argv.push("--me".to_string());
            }
            Target::AgentFullId {
                agent_full_id,
                parent_agent_instance_hierarchy,
            } => {
                argv.push("--agent-full-id".to_string());
                argv.push(agent_full_id.clone());
                if let Some(parent) = parent_agent_instance_hierarchy {
                    argv.push("--parent-agent-instance-hierarchy".to_string());
                    argv.push(parent.clone());
                }
            }
            Target::AgentInstance {
                agent_instance,
                parent_agent_instance_hierarchy,
            } => {
                argv.push("--agent-instance".to_string());
                argv.push(agent_instance.clone());
                if let Some(parent) = parent_agent_instance_hierarchy {
                    argv.push("--parent-agent-instance-hierarchy".to_string());
                    argv.push(parent.clone());
                }
            }
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// Mirrors [`Target`]. Every parent field is resolved (the handler
/// fills in the cli's own `Config.agent_instance_hierarchy` when the
/// caller omitted it).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.tags.apply.Response")]
pub enum Response {
    #[schemars(title = "Me")]
    Me {
        name: String,
        agent_instance_hierarchy: String,
    },
    #[schemars(title = "AgentFullId")]
    AgentFullId {
        name: String,
        agent_full_id: String,
        parent_agent_instance_hierarchy: String,
    },
    #[schemars(title = "AgentInstance")]
    AgentInstance {
        name: String,
        agent_instance: String,
        parent_agent_instance_hierarchy: String,
        /// `{parent}/{instance}`, or just `{instance}` when parent is
        /// empty (rootless).
        agent_instance_hierarchy: String,
    },
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("apply_by")
        .required(true)
        .multiple(false)
        .args(["me", "agent_full_id", "agent_instance"])
))]
pub struct Args {
    /// Tag name (unique). Re-using an existing tag displaces the
    /// previous binding silently.
    #[arg(long)]
    pub name: String,
    /// Bind the tag immediately to the cli's own
    /// `Config.agent_instance_hierarchy`. Forbids
    /// `--parent-agent-instance-hierarchy`.
    #[arg(long)]
    pub me: bool,
    /// Agent full id this tag is waiting on. Records a PENDING row;
    /// the next matching agent-completion auto-binds the tag.
    #[arg(long)]
    pub agent_full_id: Option<String>,
    /// Leaf agent instance id. Binds the tag immediately to
    /// `{parent}/{instance}`.
    #[arg(long)]
    pub agent_instance: Option<String>,
    /// Optional parent scope for `--agent-full-id` /
    /// `--agent-instance`. Forbidden with `--me`. When omitted, the
    /// CLI uses its own `Config.agent_instance_hierarchy`.
    #[arg(long = "parent-agent-instance-hierarchy")]
    pub parent_agent_instance_hierarchy: Option<String>,
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
        let target = match (args.me, args.agent_full_id, args.agent_instance) {
            (true, None, None) => {
                if args.parent_agent_instance_hierarchy.is_some() {
                    return Err(crate::cli::command::FromArgsError::path_parse(
                        "parent_agent_instance_hierarchy",
                        "--me forbids --parent-agent-instance-hierarchy".into(),
                    ));
                }
                Target::Me
            }
            (false, Some(id), None) => Target::AgentFullId {
                agent_full_id: id,
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
            },
            (false, None, Some(inst)) => Target::AgentInstance {
                agent_instance: inst,
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
            },
            _ => unreachable!("clap group `apply_by` ensures exactly one"),
        };
        Ok(Request {
            path_type: Path::AgentsTagsApply,
            name: args.name,
            target,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<Response, E::Error> {
    request.jq = None;
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<serde_json::Value, E::Error> {
    request.jq = Some(jq);
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;
