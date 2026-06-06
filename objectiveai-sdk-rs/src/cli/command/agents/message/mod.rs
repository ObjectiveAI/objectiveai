//! `agents message` — async handler stub.

use crate::agent::completions::message::RichContent;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message.Request")]
pub struct Request {
    pub path_type: Path,
    /// Lineage prefix to prepend to [`Self::agent_instance`]. When
    /// `None`, the CLI substitutes its own
    /// `Config.agent_instance_hierarchy` (the cli's "caller"
    /// position). Full target lineage is `"{parent}/{instance}"`.
    pub parent_agent_instance_hierarchy: Option<String>,
    /// Leaf id of the target agent. Combined with
    /// [`Self::parent_agent_instance_hierarchy`] (or the cli's
    /// caller position when that is `None`) to form the full
    /// hierarchy.
    pub agent_instance: String,
    pub message: RequestMessage,
    pub seed: Option<i64>,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    pub jq: Option<String>,
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
        let mut argv = vec![
            "agents".to_string(),
            "message".to_string(),
            self.agent_instance.clone(),
        ];
        if let Some(parent) = &self.parent_agent_instance_hierarchy {
            argv.push("--parent-agent-instance-hierarchy".to_string());
            argv.push(parent.clone());
        }
        self.message.push_flags(&mut argv);
        if let Some(seed) = self.seed {
            argv.push("--seed".to_string());
            argv.push(seed.to_string());
        }
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

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
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

/// Streamed-mode wire shape for `agents message`. Emitted as one
/// JSON-line per item on the cli's stdout when
/// `Request::dangerous_advanced.stream = Some(true)`. Untagged shape
/// is forward-compatible with [`Response`]: in stream mode, item 0 is
/// always a `Queued` or `Delivered` variant carrying the same fields
/// as the unary `Response::Queued` / `Response::Delivered`. Under the
/// `Queued` path with streaming on, item 0 is followed by zero or
/// more `Chunk` items until the spawned instance-runner's stdout
/// EOFs.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.message.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Chunk")]
    Chunk(crate::agent::completions::response::streaming::AgentCompletionChunk),
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

impl From<Response> for ResponseItem {
    fn from(r: Response) -> Self {
        match r {
            Response::Queued {
                agent_instance_hierarchy,
                response_id,
            } => ResponseItem::Queued {
                agent_instance_hierarchy,
                response_id,
            },
            Response::Delivered {
                agent_instance_hierarchy,
            } => ResponseItem::Delivered {
                agent_instance_hierarchy,
            },
        }
    }
}

#[derive(clap::Args)]
pub struct Args {
    /// Leaf id of the target agent. Combined with `--parent` (or
    /// the cli's own `Config.agent_instance_hierarchy` when
    /// `--parent` is omitted) to form the full lineage.
    pub agent_instance: String,
    /// Optional lineage prefix to prepend to `agent_instance`.
    /// When omitted, the cli substitutes its own
    /// `Config.agent_instance_hierarchy`.
    #[arg(long = "parent-agent-instance-hierarchy")]
    pub parent_agent_instance_hierarchy: Option<String>,
    #[command(flatten)]
    pub message: MessageArgs,
    /// Seed for deterministic mock responses.
    #[arg(long)]
    pub seed: Option<i64>,
    /// Raw JSON for `RequestDangerousAdvanced` (e.g. `{"stream":true}`).
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
            RequestMessage::PythonFile(args.message.python_file.unwrap())
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
            parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
            agent_instance: args.agent_instance,
            message,
            seed: args.seed,
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
