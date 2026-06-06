//! `agents tags get` — resolve a tag → agent-instance-hierarchy or
//! vice versa. Request and Response are mutually-exclusive enums
//! (one input, one output direction).

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.get.Path")]
pub enum Path {
    #[serde(rename = "agents/tags/get")]
    AgentsTagsGet,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.get.Request")]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum Request {
    #[schemars(title = "AgentInstanceHierarchy")]
    AgentInstanceHierarchy {
        path_type: Path,
        agent_instance_hierarchy: String,
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.get.Response")]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum Response {
    #[schemars(title = "AgentInstanceHierarchy")]
    AgentInstanceHierarchy {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        tag: Option<String>,
    },
    #[schemars(title = "Tag")]
    Tag {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        agent_instance_hierarchy: Option<String>,
    },
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["agents".to_string(), "tags".to_string(), "get".to_string()];
        match self {
            Request::AgentInstanceHierarchy {
                agent_instance_hierarchy,
                jq,
                ..
            } => {
                argv.push("--agent-instance-hierarchy".to_string());
                argv.push(agent_instance_hierarchy.clone());
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
pub struct Args {
    #[command(flatten)]
    pub by: ByArgs,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
pub struct ByArgs {
    /// Full agent instance hierarchy to look up.
    #[arg(long)]
    pub agent_instance_hierarchy: Option<String>,
    /// Tag name to look up.
    #[arg(long)]
    pub tag: Option<String>,
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
        if let Some(h) = args.by.agent_instance_hierarchy {
            Ok(Request::AgentInstanceHierarchy {
                path_type: Path::AgentsTagsGet,
                agent_instance_hierarchy: h,
                jq: args.jq,
            })
        } else {
            Ok(Request::Tag {
                path_type: Path::AgentsTagsGet,
                tag: args.by.tag.expect("clap group ensures one of agent-instance-hierarchy or tag is set"),
                jq: args.jq,
            })
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
