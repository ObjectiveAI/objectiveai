//! `agents message` — pure enqueue. Persists one `RichContent`
//! into the `prompts` table in `tags.sqlite` against either a
//! resolved `{parent}/{instance}` hierarchy (Direct mode) or a
//! literal tag name (Tag mode — no resolution at enqueue time).
//!
//! Same wire shape as [`super::super::message_queue::add`] — `id` of
//! the new row + the chosen target. Direct vs Tag is symmetric:
//! neither variant looks the target up at enqueue time; the API's
//! `read_message_queue` predicate picks rows up once a matching
//! hierarchy comes online (rule 1 for Direct, rules 2/3 for Tag).

use crate::agent::completions::message::RichContent;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.message.Request")]
pub struct Request {
    pub path_type: Path,
    pub target: MessageTarget,
    /// Optional payload. `None` is a no-op — nothing is written
    /// to the `message_queue` table and the response stream
    /// yields zero items. Lets scripts call `agents instances
    /// message` conditionally without branching on whether
    /// there's a payload to deliver.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub message: Option<RequestMessage>,
    pub jq: Option<String>,
}

/// Mutually-exclusive addressing for an `agents message` call.
///
/// `Direct` composes `{parent}/{agent_instance}` (parent defaults to
/// `Config.agent_instance_hierarchy` when omitted) and enqueues
/// against that hierarchy. `Tag` stores the tag name verbatim — no
/// `tags.sqlite` lookup at enqueue time; the queue reader resolves
/// the tag at dequeue time.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.instances.message.MessageTarget")]
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
    },
    #[schemars(title = "Tag")]
    Tag { agent_tag: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.message.Path")]
pub enum Path {
    #[serde(rename = "agents/instances/message")]
    AgentsInstancesMessage,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.message.RequestMessage")]
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
    /// Append the flag pair (`--simple <s>` / `--inline <json>` /
    /// `--file <path>` / `--python-inline <code>` /
    /// `--python-file <path>`) for this variant to `out`. Used by
    /// both this leaf's [`CommandRequest::into_command`] and by
    /// `agents message-queue add`'s — same wire shape, same five flags.
    pub fn push_flags(&self, out: &mut Vec<String>) {
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
        let mut argv = vec!["agents".to_string(), "instances".to_string(), "message".to_string()];
        match &self.target {
            MessageTarget::Direct {
                parent_agent_instance_hierarchy,
                agent_instance,
            } => {
                argv.push(agent_instance.clone());
                if let Some(parent) = parent_agent_instance_hierarchy {
                    argv.push("--parent-agent-instance-hierarchy".to_string());
                    argv.push(parent.clone());
                }
            }
            MessageTarget::Tag { agent_tag } => {
                argv.push("--agent-tag".to_string());
                argv.push(agent_tag.clone());
            }
        }
        if let Some(message) = &self.message {
            message.push_flags(&mut argv);
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// `id` is the row id from `tags.sqlite`'s `prompts` table. Exactly
/// one of `agent_instance_hierarchy` / `agent_tag` is set, matching
/// the chosen [`MessageTarget`] variant — `agent_instance_hierarchy`
/// is the **resolved** `{parent}/{instance}` for Direct mode.
///
/// Same shape as `agents message-queue add`'s `Response`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.message.Response")]
pub struct Response {
    pub id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_instance_hierarchy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_tag: Option<String>,
}

/// Streamed-mode wire shape is identical to the unary [`Response`] —
/// the cli emits exactly one item and exits. Kept as a type alias so
/// the SDK-side dispatch (`ResponseItem::Message(...)`) doesn't have
/// to special-case `agents instances message`.
pub type ResponseItem = Response;

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("message_target")
        .required(true)
        .multiple(false)
        .args(["agent_instance", "agent_tag"])
))]
pub struct Args {
    /// Leaf id of the target agent. Combined with `--parent` (or
    /// the cli's own `Config.agent_instance_hierarchy` when
    /// `--parent` is omitted) to form the full lineage. Mutually
    /// exclusive with `--agent-tag`.
    pub agent_instance: Option<String>,
    /// Optional lineage prefix to prepend to `agent_instance`.
    /// When omitted, the cli substitutes its own
    /// `Config.agent_instance_hierarchy`. Only valid alongside a
    /// positional `agent_instance`.
    #[arg(long = "parent-agent-instance-hierarchy", requires = "agent_instance")]
    pub parent_agent_instance_hierarchy: Option<String>,
    #[command(flatten)]
    pub message: MessageArgs,
    /// Tag name to enqueue against. Stored verbatim — the cli does
    /// NOT resolve the tag at enqueue time. Mutually exclusive with
    /// `--agent-instance`.
    #[arg(long = "agent-tag")]
    pub agent_tag: Option<String>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

#[derive(clap::Args)]
#[group(required = false, multiple = false)]
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
            Some(RequestMessage::Simple(s))
        } else if let Some(s) = args.message.inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "inline",
                    source: source.into(),
                }
            })?;
            Some(RequestMessage::Inline(v))
        } else if let Some(p) = args.message.file {
            Some(RequestMessage::File(p))
        } else if let Some(s) = args.message.python_inline {
            Some(RequestMessage::PythonInline(s))
        } else if let Some(p) = args.message.python_file {
            Some(RequestMessage::PythonFile(p))
        } else {
            None
        };
        let target = match (args.agent_instance, args.agent_tag) {
            (Some(agent_instance), None) => MessageTarget::Direct {
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
                agent_instance,
            },
            (None, Some(agent_tag)) => MessageTarget::Tag { agent_tag },
            _ => unreachable!(
                "clap group `message_target` ensures exactly one of agent_instance | agent_tag"
            ),
        };
        Ok(Self {
            path_type: Path::AgentsInstancesMessage,
            target,
            message,
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
