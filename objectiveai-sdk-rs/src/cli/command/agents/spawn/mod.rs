//! `agents spawn` — async handler stub.

use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use crate::agent::completions::message::Message;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.spawn.Request")]
pub struct Request {
    pub path_type: Path,
    pub prompt: RequestPrompt,
    pub agent: AgentSpec,
    pub seed: Option<i64>,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    pub jq: Option<String>,
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.spawn.RequestPrompt")]
pub enum RequestPrompt {
    Inline(Vec<Message>),
    Simple(String),
    File(std::path::PathBuf),
    PythonInline(String),
    PythonFile(std::path::PathBuf),
}

impl RequestPrompt {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestPrompt::Inline(msgs) => {
                out.push("--inline".to_string());
                out.push(
                    serde_json::to_string(msgs).expect("Vec<Message> serializes"),
                );
            }
            RequestPrompt::Simple(s) => {
                out.push("--simple".to_string());
                out.push(s.clone());
            }
            RequestPrompt::File(p) => {
                out.push("--file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            RequestPrompt::PythonInline(code) => {
                out.push("--python-inline".to_string());
                out.push(code.clone());
            }
            RequestPrompt::PythonFile(p) => {
                out.push("--python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["agents".to_string(), "spawn".to_string()];
        self.prompt.push_flags(&mut argv);
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
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.spawn.ResponseItem")]
pub enum ResponseItem {
    Chunk(crate::agent::completions::response::streaming::AgentCompletionChunk),
    Id(String),
}

/// Non-chunk variant of [`ResponseItem`]. Returned by the unary `execute`
/// path (with `dangerous_advanced.stream` cleared) when the cli emits a
/// single bare id string.
pub type Response = String;

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub prompt: PromptArgs,
    #[command(flatten)]
    pub agent: AgentArgs,
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
pub struct PromptArgs {
    /// Plain text — becomes one user message.
    #[arg(long)]
    pub simple: Option<String>,
    /// Inline JSON messages array.
    #[arg(long)]
    pub inline: Option<String>,
    /// Path to a JSON file containing the messages array.
    #[arg(long)]
    pub file: Option<std::path::PathBuf>,
    /// Inline Python code that produces the messages array.
    #[arg(long)]
    pub python_inline: Option<String>,
    /// Path to a Python file that produces the messages array.
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
        let prompt = if let Some(s) = args.prompt.simple {
            RequestPrompt::Simple(s)
        } else if let Some(s) = args.prompt.inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "inline",
                    source: source.into(),
                }
            })?;
            RequestPrompt::Inline(v)
        } else if let Some(p) = args.prompt.file {
            RequestPrompt::File(p)
        } else if let Some(s) = args.prompt.python_inline {
            RequestPrompt::PythonInline(s)
        } else {
            RequestPrompt::PythonFile(args.prompt.python_file.unwrap())
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
        Ok(Self { path_type: Path::AgentsSpawn,
            prompt,
            agent,
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
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
