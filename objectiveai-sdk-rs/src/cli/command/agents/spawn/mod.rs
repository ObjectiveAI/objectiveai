//! `agents spawn` — async handler stub.

use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use crate::cli::command::CommandRequest;
use crate::cli::command::agents::message::RequestMessage;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.spawn.Request")]
pub struct Request {
    pub path_type: Path,
    /// Initial user message. The CLI turns it into a single
    /// `Message::User` at the head of the `messages` array on the
    /// API call. Same wire shape as `agents message`'s
    /// `RequestMessage` — `Simple`, `Inline(RichContent)`,
    /// `File`, `PythonInline`, `PythonFile`.
    pub message: RequestMessage,
    /// How to resolve the agent for this spawn: either a directly-
    /// specified `AgentSpec` (`--agent` / `--agent-inline`) or a
    /// reference to an existing tag (`--agent-tag`). When a tag is
    /// used, the conduit injects the tag at construction time so
    /// every conduit read fires the tag-group upgrade — flipping
    /// every tag in the group to BOUND on the spawn's
    /// `agent_instance_hierarchy`.
    pub agent: AgentResolution,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    pub jq: Option<String>,
}

/// Discriminated agent-resolution mode for `agents spawn`.
///
/// - `Direct` — `--agent <ref>` / `--agent-inline <json>` paths
///   produce an `AgentSpec` directly. No tag is involved; the
///   conduit constructs without an `agent_tag`.
/// - `Tag` — `--agent-tag <name>` references an existing GROUPED
///   tag (BOUND tags are rejected — there's already a live AIH for
///   them, use `agents message` instead). The CLI handler
///   resolves the tag → `tag_groups` row, takes the row's
///   `agent_spec` to spawn, and threads the tag name into the
///   conduit so the spawn's `agent_instance_hierarchy` flips the
///   whole group to BOUND on the first conduit read.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.spawn.AgentResolution")]
pub enum AgentResolution {
    #[schemars(title = "Direct")]
    Direct { agent_spec: AgentSpec },
    #[schemars(title = "Tag")]
    Tag { agent_tag: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.spawn.Path")]
pub enum Path {
    #[serde(rename = "agents/spawn")]
    AgentsSpawn,
}

/// CLI-surface form for the `--agent` / `--agent-inline` argument: either
/// a fully resolved inline-or-remote spec, or a bare favorite name that
/// the CLI resolves to one of those at handler time. Untagged: an inline
/// agent object or a remote-path object deserializes into `Resolved`; a
/// bare JSON string lands on `Favorite`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.spawn.AgentSpec")]
pub enum AgentSpec {
    #[schemars(title = "Resolved")]
    Resolved(InlineAgentBaseWithFallbacksOrRemoteCommitOptional),
    #[schemars(title = "Favorite")]
    Favorite(String),
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["agents".to_string(), "spawn".to_string()];
        // `RequestMessage` lives in `agents::message`
        // and emits the same five flags spawn accepts.
        self.message.push_flags(&mut argv);
        // The agent argument group accepts exactly one of
        // `--agent-inline <JSON>`, `--agent <REFERENCE>`, or
        // `--agent-tag <name>`. We emit `--agent-inline` for the
        // Direct path (the Request already holds the resolved typed
        // value — the cli's parse hits the inline branch and
        // round-trips identically for both Inline and Remote
        // variants), and `--agent-tag` for the Tag path.
        match &self.agent {
            AgentResolution::Direct { agent_spec } => {
                argv.push("--agent-inline".to_string());
                argv.push(
                    serde_json::to_string(agent_spec)
                        .expect("AgentSpec serializes"),
                );
            }
            AgentResolution::Tag { agent_tag } => {
                argv.push("--agent-tag".to_string());
                argv.push(agent_tag.clone());
            }
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
#[schemars(rename = "cli.command.agents.spawn.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    /// Deterministic seed for the upstream model's RNG (mock
    /// agents in particular). Plumbed onto
    /// `AgentCompletionCreateParams.seed`. `None` here ⇒ the
    /// api picks; tests should always pin a value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.spawn.ResponseItem")]
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
    /// Raw JSON for `RequestDangerousAdvanced` (e.g.
    /// `{"stream":true,"seed":42}`).
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

/// Required user-message group. Mirrors `agents instances
/// message`'s shape: exactly one of the five flags must be set.
#[derive(clap::Args)]
#[group(required = true, multiple = false)]
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
    /// Existing tag whose `tag_groups` row provides the agent spec
    /// to spawn. The tag's parent scope is used for the new
    /// `agent_instance_hierarchy`; every tag in the same group
    /// flips to BOUND on the spawn's hierarchy via the conduit-
    /// driven upgrade on the first message-queue read.
    #[arg(long)]
    pub agent_tag: Option<String>,
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
            // Clap `required = true` on the `MessageArgs` group
            // guarantees exactly one of the five flags is set.
            RequestMessage::PythonFile(args.message.python_file.unwrap())
        };
        let agent = if let Some(s) = args.agent.agent_inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let spec: AgentSpec = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "agent_inline",
                    source: source.into(),
                }
            })?;
            AgentResolution::Direct { agent_spec: spec }
        } else if let Some(s) = args.agent.agent {
            AgentResolution::Direct {
                agent_spec: AgentSpec::Favorite(s),
            }
        } else {
            // Clap `required = true` on `AgentArgs` guarantees
            // exactly one of `--agent`, `--agent-inline`, or
            // `--agent-tag` is set.
            AgentResolution::Tag {
                agent_tag: args.agent.agent_tag.unwrap(),
            }
        };
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
        Ok(Self { path_type: Path::AgentsSpawn,
            message,
            agent,
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
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
