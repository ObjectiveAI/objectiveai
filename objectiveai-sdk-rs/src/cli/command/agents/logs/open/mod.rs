//! `agents logs read id` — fetch one logged row by its
//! `logs.messages."index"` BIGSERIAL. The handler resolves the row's
//! target table via the `logs.message_table` discriminator and
//! returns a typed [`Response`] variant carrying that table's
//! payload — no `serde_json::Value` anywhere in the public shape.
//!
//! The underlying `logs.*` tables collapse into 8 variants:
//!
//! 1. **Request-blob tiers (3)** — agent / vector / function
//!    request bodies. The JSONB column round-trips through the
//!    matching SDK `…CreateParams` type so callers see a proper
//!    typed object, not a raw blob.
//! 2. **Content payloads (5)** — `Text` / `Image` / `Audio` /
//!    `Video` / `File`. The `Text` variant subsumes all text-bearing
//!    rows (refusal, reasoning, assistant content text, tool content
//!    text, AND a tool call's `arguments`); media variants carry the
//!    SDK media type directly so MCP rendering routes through the
//!    existing `ContentBlock` projections.
//!
//! An `assistant_response_tool_calls` row reads back as `Text` (its
//! `arguments`) — the call's metadata (function_name / tool_call_id /
//! tool_call_index) is surfaced inline by `agents logs read all`. The
//! `tool_response` container head is no longer registered in
//! `logs.messages`, so it is never addressable here.

use crate::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};
use crate::agent::completions::request::AgentCompletionCreateParams;
use crate::cli::command::CommandRequest;
use crate::functions::executions::request::FunctionExecutionCreateParams;
use crate::vector::completions::request::VectorCompletionCreateParams;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.open.Request")]
pub struct Request {
    pub path_type: Path,
    pub id: i64,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.open.Path")]
pub enum Path {
    #[serde(rename = "agents/logs/open")]
    AgentsLogsOpen,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Resolved payload for one `logs.messages."index"`. Tagged by
/// `type`, snake_case discriminant. The MCP projection in
/// [`CommandResponse::into_mcp`] hands media variants over as
/// [`ContentBlock`]s and text as a bare JSON string — matching the
/// existing `agents queue read id` projection of `RichContentPart`.
/// The three request-blob variants render as JSONL with their full
/// typed `…CreateParams` body so callers can introspect request
/// metadata.
///
/// [`ContentBlock`]: crate::mcp::tool::ContentBlock
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.logs.open.Response")]
pub enum Response {
    #[schemars(title = "AgentCompletionRequest")]
    AgentCompletionRequest {
        response_id: String,
        sender_agent_instance_hierarchy: String,
        body: AgentCompletionCreateParams,
        created_at: i64,
    },
    #[schemars(title = "VectorCompletionRequest")]
    VectorCompletionRequest {
        response_id: String,
        sender_agent_instance_hierarchy: String,
        body: VectorCompletionCreateParams,
        created_at: i64,
    },
    #[schemars(title = "FunctionExecutionRequest")]
    FunctionExecutionRequest {
        response_id: String,
        sender_agent_instance_hierarchy: String,
        body: FunctionExecutionCreateParams,
        created_at: i64,
    },
    #[schemars(title = "Text")]
    Text { text: String },
    #[schemars(title = "Image")]
    Image(ImageUrl),
    #[schemars(title = "Audio")]
    Audio(InputAudio),
    #[schemars(title = "Video")]
    Video(VideoUrl),
    #[schemars(title = "File")]
    File(File),
}

/// Viewer-stream mirror of [`Request`]: the request (nested under
/// `value`, `path_type` and all) plus the broadcast stream `id`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.open.ViewerRequest")]
pub struct ViewerRequest {
    pub id: String,
    pub value: Request,
}

/// Viewer-stream mirror of [`Response`]: the response (nested under
/// `value`) plus the broadcast stream `id` and the originating request's
/// `path_type`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.open.ViewerResponse")]
pub struct ViewerResponse {
    pub id: String,
    pub path_type: Path,
    pub value: Response,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("id_required").required(true).args(["id"])))]
pub struct Args {
    /// `logs.messages."index"` — the BIGSERIAL position of the
    /// event in the cross-agent history.
    #[arg(long)]
    pub id: Option<i64>,
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
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
            path_type: Path::AgentsLogsOpen,
            id: args.id.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "id",
                    "--id is required".to_string(),
                )
            })?,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        use crate::cli::command::CommandResponse;
        match self {
            // Content payloads delegate to the existing inner-type
            // projections so they ride MCP exactly the way bare
            // `RichContentPart` does today.
            Response::Text { text } => text.into_mcp(),
            Response::Image(image_url) => image_url.into_mcp(),
            Response::Audio(input_audio) => input_audio.into_mcp(),
            Response::Video(video_url) => video_url.into_mcp(),
            Response::File(file) => file.into_mcp(),
            // Everything else: the full typed variant rides as JSONL.
            other => crate::cli::command::McpResponseItem::JSONL(
                serde_json::to_value(other).unwrap(),
            ),
        }
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

pub mod request_schema;


pub mod response_schema;
