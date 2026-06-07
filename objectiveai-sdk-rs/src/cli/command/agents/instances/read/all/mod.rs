//! `agents read all` — async handler stub.

use std::str::FromStr;

use crate::cli::command::CommandRequest;
use crate::cli::command::path_ref::tokenize;

/// One queue-read target. Either direct `(parent, instance)` (parent
/// defaults to the cli's own `Config.agent_instance_hierarchy` when
/// omitted) OR a tag name the cli resolves at handler time. Shared
/// with `agents read pending` via re-export.
///
/// Docker-style `key=value,key=value` wire form on the CLI:
///   `--target instance=L`           (direct; parent defaults to ctx)
///   `--target instance=L,parent=P`  (direct; explicit parent)
///   `--target tag=T`                (tag; cli resolves via tags.sqlite)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.instances.read.all.Target")]
pub enum Target {
    #[schemars(title = "Direct")]
    Direct {
        /// Optional lineage prefix. `None` â‡’ cli substitutes
        /// `Config.agent_instance_hierarchy`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
        /// Leaf id of the target agent.
        agent_instance: String,
    },
    #[schemars(title = "Tag")]
    Tag { agent_tag: String },
}

impl FromStr for Target {
    type Err = String;
    /// Parse a `--target` arg. Accepted keys: `instance` + optional
    /// `parent`, OR `tag` alone. `tag` is mutually exclusive with
    /// the other two keys.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tag: Option<String> = None;
        let mut parent: Option<String> = None;
        let mut instance: Option<String> = None;
        for (k, v) in tokenize(s)? {
            match k {
                "tag" => tag = Some(v.to_string()),
                "instance" => instance = Some(v.to_string()),
                "parent" => parent = Some(v.to_string()),
                other => return Err(format!("unknown key: {other}")),
            }
        }
        match (tag, instance, parent) {
            (Some(t), None, None) => Ok(Target::Tag { agent_tag: t }),
            (Some(_), _, _) => Err(
                "tag is mutually exclusive with instance and parent".to_string(),
            ),
            (None, Some(i), p) => Ok(Target::Direct {
                parent_agent_instance_hierarchy: p,
                agent_instance: i,
            }),
            (None, None, _) => Err("instance or tag is required".to_string()),
        }
    }
}

impl Target {
    /// Inverse of [`FromStr::from_str`]: emit the docker-style
    /// `key=value,key=value` wire form for round-tripping.
    pub fn into_arg_string(&self) -> String {
        match self {
            Target::Tag { agent_tag } => format!("tag={agent_tag}"),
            Target::Direct {
                parent_agent_instance_hierarchy: None,
                agent_instance,
            } => format!("instance={agent_instance}"),
            Target::Direct {
                parent_agent_instance_hierarchy: Some(p),
                agent_instance,
            } => format!("instance={agent_instance},parent={p}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.read.all.Request")]
pub struct Request {
    pub path_type: Path,
    pub targets: Vec<Target>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.read.all.Path")]
pub enum Path {
    #[serde(rename = "agents/instances/read/all")]
    AgentsInstancesReadAll,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "instances".to_string(),
            "read".to_string(),
            "all".to_string(),
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.instances.read.all.ResponseContent")]
pub enum ResponseContent {
    #[schemars(title = "One")]
    One(i64),
    #[schemars(title = "Many")]
    Many(Vec<i64>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.instances.read.all.ResponseQueueMessage")]
pub enum ResponseQueueMessage {
    #[schemars(title = "Developer")]
    Developer {
        content: ResponseContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
    },
    #[schemars(title = "System")]
    System {
        content: ResponseContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
    },
    #[schemars(title = "User")]
    User {
        content: ResponseContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
    },
    #[schemars(title = "Assistant")]
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        content: Option<ResponseContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        reasoning: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        tool_calls: Option<Vec<i64>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        refusal: Option<i64>,
    },
    #[schemars(title = "Tool")]
    Tool {
        content: ResponseContent,
        tool_call_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.instances.read.all.ResponseQueueItem")]
pub enum ResponseQueueItem {
    #[schemars(title = "AssistantResponse")]
    AssistantResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        reasoning: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        tool_calls: Option<Vec<i64>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        content: Option<ResponseContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        refusal: Option<i64>,
    },
    #[schemars(title = "ToolResponse")]
    ToolResponse {
        tool_call_id: String,
        content: ResponseContent,
    },
    #[schemars(title = "Notification")]
    Notification {
        content: ResponseContent,
    },
    #[schemars(title = "AgentCompletionRequest")]
    AgentCompletionRequest {
        messages: Vec<ResponseQueueMessage>,
    },
    #[schemars(title = "FunctionExecutionRequest")]
    FunctionExecutionRequest {
        id: i64,
    },
    #[schemars(title = "FunctionInventionRecursiveRequest")]
    FunctionInventionRecursiveRequest {
        id: i64,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.read.all.ResponseItem")]
pub struct ResponseItem {
    pub agent_id: String,
    pub items: Vec<ResponseQueueItem>,
}

#[derive(clap::Args)]
pub struct Args {
    /// One or more `--target instance=L[,parent=P]` entries. `parent`
    /// defaults to the cli's own `Config.agent_instance_hierarchy`
    /// when omitted on an individual target.
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
            path_type: Path::AgentsInstancesReadAll,
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
