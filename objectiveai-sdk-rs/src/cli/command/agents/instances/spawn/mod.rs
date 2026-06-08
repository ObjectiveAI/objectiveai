//! `agents spawn` — async handler stub.

use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use crate::cli::command::CommandRequest;
use crate::cli::command::agents::instances::message::RequestMessage;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.spawn.Request")]
pub struct Request {
    pub path_type: Path,
    /// Optional initial user message. When `Some`, the CLI turns
    /// it into a single `Message::User` at the head of the
    /// `messages` array on the API call. When `None` (e.g. on a
    /// continuation-driven re-spawn where no new user content is
    /// being delivered), the request goes out with an empty
    /// `messages` array and the API picks up state from
    /// `continuation`.
    ///
    /// Same wire shape as `agents instances message`'s
    /// `RequestMessage` — `Simple`, `Inline(RichContent)`, `File`,
    /// `PythonInline`, `PythonFile`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub message: Option<RequestMessage>,
    pub agent: AgentSpec,
    pub seed: Option<i64>,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    /// Optional tag name to bind to the spawned agent's
    /// `agent_instance_hierarchy` on first chunk. See
    /// `agents tags` for the storage model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_tag: Option<String>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.spawn.Path")]
pub enum Path {
    #[serde(rename = "agents/instances/spawn")]
    AgentsInstancesSpawn,
}

/// CLI-surface form for the `--agent` / `--agent-inline` argument: either
/// a fully resolved inline-or-remote spec, or a bare favorite name that
/// the CLI resolves to one of those at handler time. Untagged: an inline
/// agent object or a remote-path object deserializes into `Resolved`; a
/// bare JSON string lands on `Favorite`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.instances.spawn.AgentSpec")]
pub enum AgentSpec {
    #[schemars(title = "Resolved")]
    Resolved(InlineAgentBaseWithFallbacksOrRemoteCommitOptional),
    #[schemars(title = "Favorite")]
    Favorite(String),
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["agents".to_string(), "instances".to_string(), "spawn".to_string()];
        if let Some(message) = &self.message {
            // `RequestMessage` lives in `agents::instances::message`
            // and emits the same five flags spawn accepts.
            message.push_flags(&mut argv);
        }
        // The cli's `AgentArg` (from `define_inline_or_ref!`) accepts
        // either `--agent <REFERENCE>` (a `FavoriteRef` wire form, then
        // resolved to the `Remote` variant) or `--agent-inline <JSON>`
        // (deserialized directly into the SDK type). We always emit the
        // inline form because the Request already holds the resolved
        // typed value — the cli's resolve hits the inline branch and
        // round-trips identically for both Inline and Remote variants.
        argv.push("--agent-inline".to_string());
        argv.push(
            serde_json::to_string(&self.agent).expect("AgentSpec serializes"),
        );
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
        if let Some(tag) = &self.agent_tag {
            argv.push("--agent-tag".to_string());
            argv.push(tag.clone());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.spawn.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.instances.spawn.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Chunk")]
    Chunk(crate::agent::completions::response::streaming::AgentCompletionChunk),
    #[schemars(title = "Id")]
    Id(String),
}

/// Non-chunk variant of [`ResponseItem`]. Returned by the unary `execute`
/// path (with `dangerous_advanced.stream` cleared) when the cli emits a
/// single bare id string.
pub type Response = String;

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub message: MessageArgs,
    #[command(flatten)]
    pub agent: AgentArgs,
    /// Seed for deterministic mock responses.
    #[arg(long)]
    pub seed: Option<i64>,
    /// Raw JSON for `RequestDangerousAdvanced` (e.g. `{"stream":true}`).
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
    /// Optional tag name to bind on first chunk. The bound
    /// hierarchy is the spawn's resolved `agent_instance_hierarchy`.
    #[arg(long = "agent-tag")]
    pub agent_tag: Option<String>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

/// Optional user-message group. Mirrors `agents instances message`'s
/// own `MessageArgs` shape but with `required = false` — spawn
/// accepts an empty message slot.
#[derive(clap::Args)]
#[group(required = false, multiple = false)]
pub struct MessageArgs {
    /// Plain text — becomes the body of a single user message.
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
#[group(required = true, multiple = false)]
pub struct AgentArgs {
    /// Favorite-ref or remote-path string.
    #[arg(long)]
    pub agent: Option<String>,
    /// Inline JSON for the full agent definition.
    #[arg(long)]
    pub agent_inline: Option<String>,
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
        let agent = if let Some(s) = args.agent.agent_inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "agent_inline",
                    source: source.into(),
                }
            })?
        } else {
            AgentSpec::Favorite(args.agent.agent.unwrap())
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
        Ok(Self { path_type: Path::AgentsInstancesSpawn,
            message,
            agent,
            seed: args.seed,
            dangerous_advanced,
            agent_tag: args.agent_tag,
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
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
