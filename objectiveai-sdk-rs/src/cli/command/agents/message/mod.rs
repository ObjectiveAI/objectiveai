//! `agents message` — stream-aware delivery primitive.
//!
//! Resolves the target, decides whether to enqueue / race delivery
//! against a live agent / take over and spawn, all driven by
//! `dangerous_advanced.stream` (mirror of `agents instances
//! spawn`'s same flag).
//!
//! Tag resolution is the first step: a `MessageTarget::Tag` lookup
//! against `tags` either yields a BOUND hierarchy (which makes the
//! call act like a Direct target) or fails (PENDING / ABSENT), in
//! which case the call falls back to a pure enqueue.
//!
//! Once we have a resolved hierarchy, the path splits by stream
//! mode:
//!
//! - **stream=false** (default): non-acquiring lock-file check.
//!   If a live agent holds it: enqueue + race DB-delivery against
//!   lock-file release. If no live agent: re-exec ourselves as a
//!   detached subprocess with stream=true so the new process
//!   becomes the agent.
//! - **stream=true**: try to acquire the lock-file. On success:
//!   skip enqueue, run `spawn::run_multi_pass` in-process. On
//!   failure: enqueue + race DB-delivery against lock acquisition.

use crate::agent::completions::message::RichContent;
use crate::agent::completions::response::streaming::AgentCompletionChunk;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message.Request")]
pub struct Request {
    pub path_type: Path,
    pub target: MessageTarget,
    /// Required payload. The eventual enqueue / delivery / spawn
    /// always carries this exact `RichContent` as its single
    /// user message.
    pub message: RequestMessage,
    /// `Some(true)` → in-process streaming delivery / spawn.
    /// `None | Some(false)` → detached subprocess re-exec for the
    /// spawn-take-over case; the call returns the first item of
    /// that child's stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    pub jq: Option<String>,
}

/// Mutually-exclusive addressing for an `agents message` call.
///
/// `Direct` composes `{parent}/{agent_instance}` (parent defaults to
/// `Config.agent_instance_hierarchy` when omitted) and operates
/// against that hierarchy. `Tag` is resolved against the tags DB at
/// call time: a BOUND tag becomes effectively a Direct target,
/// while PENDING / ABSENT falls back to pure enqueue against the
/// tag name.
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
    /// Append the flag pair (`--simple <s>` / `--inline <json>` /
    /// `--file <path>` / `--python-inline <code>` /
    /// `--python-file <path>`) for this variant to `out`. Used by
    /// both this leaf's [`CommandRequest::into_command`] and by
    /// `agents queue add`'s — same wire shape, same five flags.
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

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["agents".to_string(), "message".to_string()];
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
        self.message.push_flags(&mut argv);
        if let Some(advanced) = &self.dangerous_advanced {
            argv.push("--dangerous-advanced".to_string());
            argv.push(
                serde_json::to_string(advanced)
                    .expect("RequestDangerousAdvanced serializes"),
            );
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// Unary response (stream=false). Exactly one of these per call.
/// Internally tagged via `type`; bare unit variant `Delivered`
/// serializes as `{"type":"delivered"}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.message.Response")]
pub enum Response {
    /// The queue row reached a live agent (the API's conduit ran
    /// `clear_by_ids` on it) before any other race finalized.
    Delivered,
    /// The target's tag wasn't bound at call time (PENDING /
    /// ABSENT). The message was deferred into the queue.
    Enqueued {
        id: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        agent_instance_hierarchy: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        agent_tag: Option<String>,
    },
    /// The stream=false path re-execed itself as a detached
    /// subprocess (stream=true) and the subprocess yielded a
    /// `ResponseItem::Id` first. Same payload as spawn's
    /// `ResponseItem::Id(String)` — the bare
    /// `agent_instance_hierarchy` string the runner just minted.
    Id { agent_instance_hierarchy: String },
}

/// Streamed response (stream=true). The cli yields a sequence of
/// these. Same `Delivered` / `Enqueued` / `Id` first-item
/// semantics as [`Response`]; the spawn-take-over branch adds
/// streaming `Chunk` items after the initial `Id`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.message.ResponseItem")]
pub enum ResponseItem {
    Delivered,
    Enqueued {
        id: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        agent_instance_hierarchy: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        agent_tag: Option<String>,
    },
    Id { agent_instance_hierarchy: String },
    /// Newtype-of-struct under an internally-tagged enum: the
    /// chunk's own fields land at the top level of the JSON, with
    /// `"type":"chunk"` injected. Wire shape equivalent to spawn's
    /// `ResponseItem::Chunk(AgentCompletionChunk)` plus the `type`
    /// discriminator.
    Chunk(AgentCompletionChunk),
}

impl From<Response> for ResponseItem {
    /// Lift the unary [`Response`] into the streaming
    /// [`ResponseItem`] shape. Lossless — every `Response`
    /// variant maps 1-to-1 onto a `ResponseItem` variant of the
    /// same name; streaming-only variants (`Chunk`) are never
    /// produced from a `Response`.
    fn from(r: Response) -> Self {
        match r {
            Response::Delivered => ResponseItem::Delivered,
            Response::Enqueued {
                id,
                agent_instance_hierarchy,
                agent_tag,
            } => ResponseItem::Enqueued {
                id,
                agent_instance_hierarchy,
                agent_tag,
            },
            Response::Id {
                agent_instance_hierarchy,
            } => ResponseItem::Id {
                agent_instance_hierarchy,
            },
        }
    }
}

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
    /// Raw JSON for [`RequestDangerousAdvanced`] (e.g.
    /// `{"stream":true}`).
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
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
            // Clap `required = true` on `MessageArgs` guarantees
            // exactly one of the five flags is set.
            RequestMessage::PythonFile(args.message.python_file.unwrap())
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
        let dangerous_advanced = if let Some(s) = args.dangerous_advanced {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "dangerous_advanced",
                    source: source.into(),
                }
            })?;
            Some(v)
        } else {
            None
        };
        Ok(Self {
            path_type: Path::AgentsMessage,
            target,
            message,
            dangerous_advanced,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.jq = None;
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.jq = Some(jq);
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
    request.jq = None;
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
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
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
