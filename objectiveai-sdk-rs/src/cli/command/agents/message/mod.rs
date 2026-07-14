//! `agents message` — unary delivery primitive with a total FIFO
//! order.
//!
//! Addresses an agent with the same shape as `agents spawn` (ref /
//! tag / instance). A plain ref execs a detached `agents spawn`
//! child carrying the message inline and returns the child's first
//! item (`Id`). An instance or tag ALWAYS enqueues first — the queue
//! row's id fixes the delivery position at commit time — then races
//! the agent's lock; winning it execs a WAKE-UP spawn child (EMPTY
//! message, the lock TRANSFERRED into it) that drains the queue
//! oldest-id-first. Either way the call resolves `Delivered` when
//! its own row is consumed, so delivery order is always enqueue
//! order. For fire-and-forget parking without the race, see
//! `agents enqueue`.

use crate::agent::completions::message::RichContent;
use crate::cli::command::CommandRequest;
use crate::cli::command::agents::selector::AgentSelector;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message.Request")]
pub struct Request {
    pub path_type: Path,
    /// Who receives the message — same shape as `agents spawn`'s
    /// `agent`: a direct ref spawns a fresh agent carrying this
    /// message; a tag resolves at call time (BOUND → its live
    /// hierarchy, GROUPED → first message spawns the agent and
    /// upgrades the group, ABSENT → error); an instance targets an
    /// existing hierarchy.
    pub agent: AgentSelector,
    /// Required payload. The eventual enqueue / delivery / spawn
    /// always carries this exact `RichContent` as its single
    /// user message.
    pub message: RequestMessage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
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
    /// Deterministic seed for the upstream model's RNG. Forwarded
    /// onto the spawn child's `AgentCompletionCreateParams.seed`.
    /// `None` here ⇒ the api picks; tests should always pin a
    /// value to keep continuation turns reproducible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Unary response. Exactly one of these per call. Internally tagged
/// via `type`; bare unit variant `Delivered` serializes as
/// `{"type":"delivered"}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.message.Response")]
pub enum Response {
    /// The queue row reached a live agent (its row flipped to
    /// inactive — the API stamped its id onto an assistant chunk's
    /// `request_message_ids`). The only resolution for instance and
    /// tag targets: whether the handler found the agent live or
    /// spawned it, the message rides the queue and this fires when
    /// its own row is consumed.
    #[schemars(title = "Delivered")]
    Delivered,
    /// Plain-ref targets only: the handler execed a detached
    /// `agents spawn` child carrying the message inline and the
    /// child yielded its `Id` first item — the bare
    /// `agent_instance_hierarchy` the runner just minted.
    #[schemars(title = "Id")]
    Id { agent_instance_hierarchy: String },
}

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub agent: crate::cli::command::agents::selector::AgentSelectorArgs,
    #[command(flatten)]
    pub message: MessageArgs,
    /// Raw JSON for [`RequestDangerousAdvanced`] (e.g.
    /// `{"seed":42}`).
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
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
        let agent = AgentSelector::try_from(args.agent)?;
        let dangerous_advanced: Option<RequestDangerousAdvanced> =
            if let Some(s) = args.dangerous_advanced {
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
            agent,
            message,
            dangerous_advanced,
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

/// One `/listen` broadcast run of `agents message`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
