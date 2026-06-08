//! `agents tags lookup` — resolve a tag → agent-instance-hierarchy or
//! vice versa. Request and Response are mutually-exclusive enums
//! (one input, one output direction).

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.lookup.Path")]
pub enum Path {
    #[serde(rename = "agents/tags/lookup")]
    AgentsTagsLookup,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.lookup.Request")]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum Request {
    #[schemars(title = "AgentInstanceHierarchy")]
    AgentInstanceHierarchy {
        path_type: Path,
        /// Lineage prefix to prepend to [`Self::agent_instance`].
        /// When `None`, the CLI substitutes its own
        /// `Config.agent_instance_hierarchy`. Full hierarchy is
        /// `"{parent}/{instance}"`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
        /// Leaf id of the target agent.
        agent_instance: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        jq: Option<String>,
    },
    #[schemars(title = "Tag")]
    Tag {
        path_type: Path,
        tag: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        jq: Option<String>,
    },
}

impl Request {
    fn jq_mut(&mut self) -> &mut Option<String> {
        match self {
            Request::AgentInstanceHierarchy { jq, .. } => jq,
            Request::Tag { jq, .. } => jq,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.lookup.Response")]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum Response {
    /// A hierarchy can carry multiple tags (the schema's PK is the
    /// `name`, not the hierarchy). All matching BOUND tags are
    /// returned, newest-bound first.
    #[schemars(title = "AgentInstanceHierarchy")]
    AgentInstanceHierarchy { tags: Vec<String> },
    /// A successful tag → state lookup. Flattens the 2-state
    /// status onto the same JSON object — yielding e.g.
    /// `{"by":"tag","state":"bound","agent_instance_hierarchy":"…"}`.
    #[schemars(title = "Tag")]
    Tag {
        #[serde(flatten)]
        state: LookupState,
    },
    /// The looked-up tag is not registered. Hoisted to a top-level
    /// variant (rather than as a `LookupState::Absent` nested in
    /// `Tag`) so the wire shape says "no such tag" instead of
    /// "the tag exists with state absent".
    #[schemars(title = "Absent")]
    Absent,
}

/// 2-state result of a successful tag-name lookup. `Grouped`
/// surfaces the tag's `tag_groups` membership — the group id, the
/// resolved `AgentSpec` the group carries, and the parent lineage
/// — so callers can see what would happen on spawn-by-tag without
/// firing one. The third "not registered" possibility is
/// represented at the [`Response`] level via [`Response::Absent`],
/// not as a variant here.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.lookup.LookupState")]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum LookupState {
    #[schemars(title = "Bound")]
    Bound { agent_instance_hierarchy: String },
    #[schemars(title = "Grouped")]
    Grouped {
        tag_group_id: i64,
        agent_spec: super::super::instances::spawn::AgentSpec,
        parent_agent_instance_hierarchy: String,
    },
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["agents".to_string(), "tags".to_string(), "lookup".to_string()];
        match self {
            Request::AgentInstanceHierarchy {
                parent_agent_instance_hierarchy,
                agent_instance,
                jq,
                ..
            } => {
                argv.push(agent_instance.clone());
                if let Some(parent) = parent_agent_instance_hierarchy {
                    argv.push("--parent-agent-instance-hierarchy".to_string());
                    argv.push(parent.clone());
                }
                if let Some(jq) = jq {
                    argv.push("--jq".to_string());
                    argv.push(jq.clone());
                }
            }
            Request::Tag { tag, jq, .. } => {
                argv.push("--tag".to_string());
                argv.push(tag.clone());
                if let Some(jq) = jq {
                    argv.push("--jq".to_string());
                    argv.push(jq.clone());
                }
            }
        }
        argv
    }
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("lookup_by")
        .required(true)
        .multiple(false)
        .args(["agent_instance", "tag"])
))]
pub struct Args {
    /// Leaf id of the target agent. Combined with `--parent` (or
    /// the cli's own `Config.agent_instance_hierarchy` when omitted)
    /// to form the full lineage. Mutually exclusive with `--tag`.
    pub agent_instance: Option<String>,
    /// Optional lineage prefix to prepend to `agent_instance`.
    /// When omitted, the cli substitutes its own
    /// `Config.agent_instance_hierarchy`. Only valid alongside
    /// `agent_instance`.
    #[arg(long = "parent-agent-instance-hierarchy", requires = "agent_instance")]
    pub parent_agent_instance_hierarchy: Option<String>,
    /// Tag name to look up. Mutually exclusive with `agent_instance`
    /// and `--parent-agent-instance-hierarchy`.
    #[arg(long)]
    pub tag: Option<String>,
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
        match (args.agent_instance, args.tag) {
            (Some(agent_instance), None) => Ok(Request::AgentInstanceHierarchy {
                path_type: Path::AgentsTagsLookup,
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
                agent_instance,
                jq: args.jq,
            }),
            (None, Some(tag)) => Ok(Request::Tag {
                path_type: Path::AgentsTagsLookup,
                tag,
                jq: args.jq,
            }),
            _ => unreachable!(
                "clap group `lookup_by` ensures exactly one of agent_instance | tag"
            ),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<Response, E::Error> {
    *request.jq_mut() = None;
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<serde_json::Value, E::Error> {
    *request.jq_mut() = Some(jq);
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
