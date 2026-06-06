//! `agents queue read id` — fetch one piece of queued content by
//! its `prompt_contents.id`.
//!
//! `agents queue list` emits role-keyed `ResponseQueueMessage`s
//! whose content fields are `i64` references into the per-kind
//! content tables (`prompt_texts`, `prompt_images`, …). This leaf
//! takes one such id and returns the row's typed payload, tagged
//! by `type` so callers can dispatch without sniffing fields.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.read.id.Request")]
pub struct Request {
    pub path_type: Path,
    pub id: i64,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.read.id.Path")]
pub enum Path {
    #[serde(rename = "agents/queue/read/id")]
    AgentsQueueReadId,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "queue".to_string(),
            "read".to_string(),
            "id".to_string(),
            self.id.to_string(),
        ];
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// One typed content row. Variants map 1:1 to `prompt_contents.kind`.
/// Tagged by `type` (mirroring `RichContentPart`'s shape) — the wire
/// form for an image is e.g.
/// `{"type":"image","image_url":{"url":"data:image/png;base64,…"}}`.
///
/// `reasoning` and `refusal` carry plain strings because the SDK
/// `AssistantMessage` defines both as `Option<String>` — there's no
/// envelope to preserve. `tool_call` is the structured
/// [`crate::agent::completions::message::AssistantToolCall`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.queue.read.id.Response")]
pub enum Response {
    #[schemars(title = "Text")]
    Text { text: String },
    #[schemars(title = "Image")]
    Image { image_url: crate::agent::completions::message::ImageUrl },
    #[schemars(title = "Audio")]
    Audio { input_audio: crate::agent::completions::message::InputAudio },
    #[schemars(title = "Video")]
    Video { video_url: crate::agent::completions::message::VideoUrl },
    #[schemars(title = "File")]
    File { file: crate::agent::completions::message::File },
    #[schemars(title = "Reasoning")]
    Reasoning { reasoning: String },
    #[schemars(title = "Refusal")]
    Refusal { refusal: String },
    #[schemars(title = "ToolCall")]
    ToolCall { tool_call: crate::agent::completions::message::AssistantToolCall },
}

#[derive(clap::Args)]
pub struct Args {
    /// `prompt_contents.id` of the content row to fetch.
    pub id: i64,
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
        Ok(Self {
            path_type: Path::AgentsQueueReadId,
            id: args.id,
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
        // Delegate to the inner type's `CommandResponse` impl when
        // there is one — media types pick up `Media(ContentBlock::…)`,
        // `String` picks up `Value::String`. `Reasoning` / `Refusal`
        // unwrap to their bare strings (also pick up `Value::String`).
        // `ToolCall` falls back to JSONL of the full struct since
        // there's no media analogue for tool calls.
        match self {
            Response::Text { text } => text.into_mcp(),
            Response::Image { image_url } => image_url.into_mcp(),
            Response::Audio { input_audio } => input_audio.into_mcp(),
            Response::Video { video_url } => video_url.into_mcp(),
            Response::File { file } => file.into_mcp(),
            Response::Reasoning { reasoning } => reasoning.into_mcp(),
            Response::Refusal { refusal } => refusal.into_mcp(),
            Response::ToolCall { tool_call } => crate::cli::command::McpResponseItem::JSONL(
                serde_json::to_value(&tool_call).unwrap(),
            ),
        }
    }
}

pub mod request_schema;

pub mod response_schema;
