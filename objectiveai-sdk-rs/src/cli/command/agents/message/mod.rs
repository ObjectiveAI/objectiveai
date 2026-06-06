//! `agents message` — async handler stub.

use crate::agent::completions::message::RichContent;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message.Request")]
pub struct Request {
    pub path_type: Path,
    pub target: MessageTarget,
    pub message: RequestMessage,
    pub seed: Option<i64>,
    pub jq: Option<String>,
}

/// Mutually-exclusive addressing for an `agents message` call.
///
/// `Direct` is the default — the CLI composes
/// `{parent}/{agent_instance}` (parent defaults to
/// `Config.agent_instance_hierarchy` when omitted) and optionally
/// binds `agent_tag` to that hierarchy as a side effect.
///
/// `Tag` resolves the named tag to its bound hierarchy via the
/// tags DB and addresses that hierarchy directly. PENDING and
/// ABSENT tags are rejected with structured errors at handler time.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.message.MessageTarget")]
pub enum MessageTarget {
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
        /// Optional tag to bind to the resolved
        /// `{parent}/{agent_instance}` hierarchy.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        agent_tag: Option<String>,
    },
    #[schemars(title = "Tag")]
    Tag { agent_tag: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message.Path")]
pub enum Path {
    #[serde(rename = "agents/message")]
    AgentsMessage,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message.RequestMessage")]
pub enum RequestMessage {
    #[schemars(title = "Inline")]
    Inline(RichContent),
    #[schemars(title = "Simple")]
    Simple(String),
    #[schemars(title = "File")]
    File(std::path::PathBuf),
    #[schemars(title = "PythonInline")]
    PythonInline(String),
    #[schemars(title = "PythonFile")]
    PythonFile(std::path::PathBuf),
}

impl RequestMessage {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestMessage::Inline(rich) => {
                out.push("--inline".to_string());
                out.push(
                    serde_json::to_string(rich)
                        .expect("RichContent serializes to JSON cleanly"),
                );
            }
            RequestMessage::Simple(s) => {
                out.push("--simple".to_string());
                out.push(s.clone());
            }
            RequestMessage::File(p) => {
                out.push("--file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            RequestMessage::PythonInline(code) => {
                out.push("--python-inline".to_string());
                out.push(code.clone());
            }
            RequestMessage::PythonFile(p) => {
                out.push("--python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["agents".to_string(), "message".to_string()];
        match &self.target {
            MessageTarget::Direct {
                parent_agent_instance_hierarchy,
                agent_instance,
                agent_tag,
            } => {
                argv.push(agent_instance.clone());
                if let Some(parent) = parent_agent_instance_hierarchy {
                    argv.push("--parent-agent-instance-hierarchy".to_string());
                    argv.push(parent.clone());
                }
                if let Some(tag) = agent_tag {
                    argv.push("--agent-tag".to_string());
                    argv.push(tag.clone());
                }
            }
            MessageTarget::Tag { agent_tag } => {
                argv.push("--agent-tag".to_string());
                argv.push(agent_tag.clone());
            }
        }
        self.message.push_flags(&mut argv);
        if let Some(seed) = self.seed {
            argv.push("--seed".to_string());
            argv.push(seed.to_string());
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
#[schemars(rename = "cli.command.agents.message.Response")]
pub enum Response {
    #[schemars(title = "Queued")]
    Queued {
        agent_instance_hierarchy: String,
        response_id: String,
    },
    #[schemars(title = "Delivered")]
    Delivered {
        agent_instance_hierarchy: String,
    },
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("message_target")
        .required(true)
        .multiple(true)
        .args(["agent_instance", "agent_tag"])
))]
pub struct Args {
    /// Leaf id of the target agent. Combined with `--parent` (or
    /// the cli's own `Config.agent_instance_hierarchy` when
    /// `--parent` is omitted) to form the full lineage. Optional
    /// when `--agent-tag` is set alone (tag-resolution mode);
    /// required otherwise.
    pub agent_instance: Option<String>,
    /// Optional lineage prefix to prepend to `agent_instance`.
    /// When omitted, the cli substitutes its own
    /// `Config.agent_instance_hierarchy`. Only valid alongside a
    /// positional `agent_instance`.
    #[arg(long = "parent-agent-instance-hierarchy", requires = "agent_instance")]
    pub parent_agent_instance_hierarchy: Option<String>,
    #[command(flatten)]
    pub message: MessageArgs,
    /// Seed for deterministic mock responses.
    #[arg(long)]
    pub seed: Option<i64>,
    /// Tag to apply to the resolved hierarchy (Direct mode, when
    /// `agent_instance` is also set), or to look up to find the
    /// target hierarchy (Tag mode, when `agent_instance` is
    /// omitted).
    #[arg(long = "agent-tag")]
    pub agent_tag: Option<String>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
pub struct MessageArgs {
    /// Plain text — becomes one user message.
    #[arg(long)]
    pub simple: Option<String>,
    /// Inline JSON `RichContent`.
    #[arg(long)]
    pub inline: Option<String>,
    /// Path to a JSON file containing the rich content.
    #[arg(long)]
    pub file: Option<std::path::PathBuf>,
    /// Inline Python code that produces the rich content.
    #[arg(long)]
    pub python_inline: Option<String>,
    /// Path to a Python file that produces the rich content.
    #[arg(long)]
    pub python_file: Option<std::path::PathBuf>,
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
        let message = if let Some(s) = args.message.simple {
            RequestMessage::Simple(s)
        } else if let Some(s) = args.message.inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "inline",
                    source: source.into(),
                }
            })?;
            RequestMessage::Inline(v)
        } else if let Some(p) = args.message.file {
            RequestMessage::File(p)
        } else if let Some(s) = args.message.python_inline {
            RequestMessage::PythonInline(s)
        } else {
            RequestMessage::PythonFile(args.message.python_file.unwrap())
        };
        let target = match (args.agent_instance, args.agent_tag) {
            (Some(agent_instance), agent_tag) => MessageTarget::Direct {
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
                agent_instance,
                agent_tag,
            },
            (None, Some(agent_tag)) => MessageTarget::Tag { agent_tag },
            (None, None) => unreachable!(
                "clap group `message_target` ensures at least one of agent_instance | agent_tag"
            ),
        };
        Ok(Self {
            path_type: Path::AgentsMessage,
            target,
            message,
            seed: args.seed,
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
