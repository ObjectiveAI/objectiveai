//! `agents queue add` — defer a prompt to a target agent.
//!
//! Stores the prompt in `prompt_queue.sqlite` against either a
//! resolved `agent_instance_hierarchy` (Direct mode) or a literal
//! `agent_tag` (Tag mode — no resolution at enqueue time; the
//! future reader will resolve at dequeue time). The two target
//! modes are mutually exclusive at the clap layer and at the enum
//! variant layer.
//!
//! This is the write-only first slice of #211. No dequeue/flush
//! exists yet — rows are persisted until a future reader picks
//! them up.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.add.Request")]
pub struct Request {
    pub path_type: Path,
    pub target: Target,
    pub prompt: super::super::spawn::RequestPrompt,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.add.Path")]
pub enum Path {
    #[serde(rename = "agents/queue/add")]
    AgentsQueueAdd,
}

/// Mutually-exclusive target. `Direct` composes
/// `{parent}/{agent_instance}` at handler time (parent defaults to
/// the cli's own `Config.agent_instance_hierarchy`). `Tag` stores
/// the tag name verbatim — no `tags.sqlite` lookup at enqueue time.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.queue.add.Target")]
pub enum Target {
    #[schemars(title = "Direct")]
    Direct {
        /// Lineage prefix to prepend to `agent_instance`. When
        /// `None`, the CLI substitutes its own
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

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "queue".to_string(),
            "add".to_string(),
        ];
        match &self.target {
            Target::Direct {
                parent_agent_instance_hierarchy,
                agent_instance,
            } => {
                argv.push(agent_instance.clone());
                if let Some(parent) = parent_agent_instance_hierarchy {
                    argv.push("--parent-agent-instance-hierarchy".to_string());
                    argv.push(parent.clone());
                }
            }
            Target::Tag { agent_tag } => {
                argv.push("--agent-tag".to_string());
                argv.push(agent_tag.clone());
            }
        }
        self.prompt.push_flags(&mut argv);
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// `id` is the row id from `prompt_queue.sqlite`. Exactly one of
/// `agent_instance_hierarchy` / `agent_tag` is set, matching the
/// chosen [`Target`] variant — `agent_instance_hierarchy`
/// is the **resolved** `{parent}/{instance}` for Direct mode.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.add.Response")]
pub struct Response {
    pub id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_instance_hierarchy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_tag: Option<String>,
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("queue_add_target")
        .required(true)
        .multiple(false)
        .args(["agent_instance", "agent_tag"])
))]
pub struct Args {
    /// Leaf id of the target agent. Combined with `--parent` (or
    /// the cli's own `Config.agent_instance_hierarchy` when
    /// omitted) to form the full lineage. Mutually exclusive with
    /// `--agent-tag`.
    pub agent_instance: Option<String>,
    /// Optional lineage prefix to prepend to `agent_instance`.
    /// Only valid alongside `agent_instance`.
    #[arg(long = "parent-agent-instance-hierarchy", requires = "agent_instance")]
    pub parent_agent_instance_hierarchy: Option<String>,
    /// Tag name to enqueue against. Stored verbatim — the cli does
    /// NOT resolve the tag at enqueue time. Mutually exclusive with
    /// `agent_instance` and `--parent-agent-instance-hierarchy`.
    #[arg(long = "agent-tag")]
    pub agent_tag: Option<String>,
    /// Prompt input (one of `--simple` / `--inline` / `--file` /
    /// `--python-inline` / `--python-file`). Required.
    #[command(flatten)]
    pub prompt: super::super::spawn::PromptArgs,
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
        // Same branching as `agents spawn`'s TryFrom — normalise the
        // five mutually-exclusive PromptArgs fields into one
        // RequestPrompt variant.
        let prompt = if let Some(s) = args.prompt.simple {
            super::super::spawn::RequestPrompt::Simple(s)
        } else if let Some(s) = args.prompt.inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "inline",
                    source: source.into(),
                }
            })?;
            super::super::spawn::RequestPrompt::Inline(v)
        } else if let Some(p) = args.prompt.file {
            super::super::spawn::RequestPrompt::File(p)
        } else if let Some(s) = args.prompt.python_inline {
            super::super::spawn::RequestPrompt::PythonInline(s)
        } else {
            super::super::spawn::RequestPrompt::PythonFile(args.prompt.python_file.unwrap())
        };
        let target = match (args.agent_instance, args.agent_tag) {
            (Some(agent_instance), None) => Target::Direct {
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
                agent_instance,
            },
            (None, Some(agent_tag)) => Target::Tag { agent_tag },
            _ => unreachable!(
                "clap group `queue_add_target` ensures exactly one of agent_instance | agent_tag"
            ),
        };
        Ok(Self {
            path_type: Path::AgentsQueueAdd,
            target,
            prompt,
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
