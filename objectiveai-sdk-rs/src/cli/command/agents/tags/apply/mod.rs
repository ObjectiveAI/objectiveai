//! `agents tags apply` — bind a tag to an agent-instance-hierarchy,
//! to a fresh `tag_groups` row, or to another tag's resolution.
//!
//! Three target shapes, mutually exclusive via the `apply_by` clap
//! group:
//!
//! - `--agent-instance <leaf>` — BOUND immediately to
//!   `{parent}/{leaf}`. Optional
//!   `--parent-agent-instance-hierarchy` defaults to the cli's own
//!   `Config.agent_instance_hierarchy`.
//! - `--agent <ref>` / `--agent-inline <json>` — creates a fresh
//!   `tag_groups` row carrying the resolved `AgentSpec` + parent.
//!   The new tag points at that group. Spawning by this tag (via
//!   `agents spawn --agent-tag <name>`) uses the group's
//!   AgentSpec. Optional parent defaults to ctx own.
//! - `--agent-tag <existing>` — clones the existing tag's
//!   resolution under the new name. BOUND sources copy the
//!   hierarchy; GROUPED sources join the same `tag_group`. Forbids
//!   `--parent-agent-instance-hierarchy`.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.apply.Request")]
pub struct Request {
    pub path_type: Path,
    pub name: String,
    pub target: Target,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.tags.apply.Target")]
pub enum Target {
    /// `tag → AIH`. Storage gets the resolved AIH =
    /// `{parent}/{agent_instance}`.
    #[schemars(title = "AgentInstance")]
    AgentInstance {
        agent_instance: String,
        /// Optional parent scope. `None` ⇒ cli substitutes
        /// `Config.agent_instance_hierarchy`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
    },
    /// `tag → new tag_group`. Creates a fresh row in `tag_groups`
    /// carrying the resolved AgentSpec + parent, then a `tags`
    /// row pointing at it. Multiple subsequent
    /// `Target::AgentTag` applies can join the new tag's group.
    #[schemars(title = "Agent")]
    Agent {
        agent_spec: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
        /// Optional parent scope. `None` ⇒ cli substitutes
        /// `Config.agent_instance_hierarchy`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
    },
    /// Clone an existing tag's resolution under `--name`. BOUND
    /// source copies the hierarchy; GROUPED source joins the same
    /// `tag_group`. `parent_agent_instance_hierarchy` is forbidden
    /// here (the source's parent — via its group — is inherited).
    #[schemars(title = "AgentTag")]
    AgentTag { agent_tag: String },
}

/// Snapshot of the source tag's resolution at the moment
/// `--agent-tag` looked it up. The handler reproduces this state
/// under the caller's `--name`. Self-contained on this leaf — not
/// shared with `tags::lookup`'s `LookupState` so each leaf's
/// schema stays decoupled.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.tags.apply.AgentTagResolution")]
pub enum AgentTagResolution {
    #[schemars(title = "Bound")]
    Bound { agent_instance_hierarchy: String },
    #[schemars(title = "Grouped")]
    Grouped {
        tag_group_id: i64,
        agent_spec: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
        parent_agent_instance_hierarchy: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.apply.Path")]
pub enum Path {
    #[serde(rename = "agents/tags/apply")]
    AgentsTagsApply,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Apply response. Each variant carries the freshly-applied state
/// so callers don't need a follow-up lookup.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.tags.apply.Response")]
pub enum Response {
    #[schemars(title = "AgentInstance")]
    AgentInstance {
        name: String,
        agent_instance: String,
        parent_agent_instance_hierarchy: String,
        /// `{parent}/{agent_instance}`.
        agent_instance_hierarchy: String,
    },
    #[schemars(title = "Agent")]
    Agent {
        name: String,
        tag_group_id: i64,
        agent_spec: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
        parent_agent_instance_hierarchy: String,
    },
    /// Wire shape mirrors the resolved source state:
    /// `{"by":"agent_tag","name":...,"agent_tag":...,"state":"bound"|"grouped", …}`.
    #[schemars(title = "AgentTag")]
    AgentTag {
        name: String,
        agent_tag: String,
        #[serde(flatten)]
        resolved: AgentTagResolution,
    },
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("apply_by")
        .required(true)
        .multiple(false)
        .args(["agent_instance", "agent", "agent_inline", "agent_tag"])
))]
pub struct Args {
    /// Tag name (unique). Re-using an existing tag displaces the
    /// previous binding silently.
    #[arg(long)]
    pub name: String,
    /// Bind the tag immediately to `{parent}/{agent_instance}`.
    #[arg(long)]
    pub agent_instance: Option<String>,
    /// Resolved agent reference (docker-style `key=value,…`).
    /// Mutually exclusive with `--agent-inline`.
    #[arg(long)]
    pub agent: Option<String>,
    /// Inline JSON for the full agent definition. Mutually
    /// exclusive with `--agent`.
    #[arg(long)]
    pub agent_inline: Option<String>,
    /// Existing tag whose resolution gets cloned under `--name`.
    /// Forbids `--parent-agent-instance-hierarchy`.
    #[arg(long)]
    pub agent_tag: Option<String>,
    /// Optional parent scope. Allowed with `--agent-instance` and
    /// the `--agent` / `--agent-inline` pair; forbidden with
    /// `--agent-tag`. When omitted, the cli substitutes its own
    /// `Config.agent_instance_hierarchy`.
    #[arg(long = "parent-agent-instance-hierarchy")]
    pub parent_agent_instance_hierarchy: Option<String>,
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
        let target = match (
            args.agent_instance,
            args.agent,
            args.agent_inline,
            args.agent_tag,
        ) {
            (Some(inst), None, None, None) => Target::AgentInstance {
                agent_instance: inst,
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
            },
            (None, Some(s), None, None) => {
                use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
                let path: crate::RemotePathCommitOptional = s
                    .parse()
                    .map_err(|e| crate::cli::command::FromArgsError::path_parse("agent", e))?;
                Target::Agent {
                    agent_spec: InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(path),
                    parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
                }
            }
            (None, None, Some(s), None) => {
                let mut de = serde_json::Deserializer::from_str(&s);
                let spec = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                    crate::cli::command::FromArgsError {
                        field: "agent_inline",
                        source: source.into(),
                    }
                })?;
                Target::Agent {
                    agent_spec: spec,
                    parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
                }
            }
            (None, None, None, Some(tag)) => {
                if args.parent_agent_instance_hierarchy.is_some() {
                    return Err(crate::cli::command::FromArgsError::path_parse(
                        "parent_agent_instance_hierarchy",
                        "--agent-tag forbids --parent-agent-instance-hierarchy".into(),
                    ));
                }
                Target::AgentTag { agent_tag: tag }
            }
            _ => unreachable!(
                "clap group `apply_by` ensures exactly one of agent_instance | agent | agent_inline | agent_tag"
            ),
        };
        Ok(Request {
            path_type: Path::AgentsTagsApply,
            name: args.name,
            target,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<Response, E::Error> {
    request.base.clear_transform();
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<serde_json::Value, E::Error> {
    request.base.set_transform(transform);
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
