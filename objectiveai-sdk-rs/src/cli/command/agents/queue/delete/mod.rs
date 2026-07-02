//! `agents queue delete` — manually drop one queued prompt
//! by its `prompts.id` (the id surfaced by `agents queue
//! list`). Returns the deleted row's metadata + content so callers
//! can confirm exactly which item was dropped.
//!
//! Cascade on `prompt_contents.prompt_id` sweeps every per-kind
//! content row inside the same transaction, so the delete is
//! atomic.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.delete.Request")]
pub struct Request {
    pub path_type: Path,
    pub id: i64,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.delete.Path")]
pub enum Path {
    #[serde(rename = "agents/queue/delete")]
    AgentsQueueDelete,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// What was deleted. Carries every column of the original
/// `prompts` row so the caller can confirm the drop:
/// exactly one of `agent_instance_hierarchy` / `agent_tag` is set
/// (matching the original target), `enqueued_at` is the original
/// unix-seconds timestamp, and `content` is the reconstructed
/// `RichContent` body.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.delete.Response")]
pub struct Response {
    pub id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_instance_hierarchy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_tag: Option<String>,
    /// Idempotency token, if the dropped row had one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub key: Option<String>,
    /// RFC3339 timestamp the dropped row was enqueued at.
    pub enqueued_at: String,
    pub content: crate::agent::completions::message::RichContent,
}

/// Viewer-stream mirror of [`Request`]: the request (nested under
/// `value`, `path_type` and all) plus the broadcast stream `id`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.delete.ViewerRequest")]
pub struct ViewerRequest {
    pub id: String,
    pub value: Request,
}

/// Viewer-stream mirror of [`Response`]: the response (nested under
/// `value`) plus the broadcast stream `id` and the originating request's
/// `path_type`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.delete.ViewerResponse")]
pub struct ViewerResponse {
    pub id: String,
    pub path_type: Path,
    pub value: Response,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("id_required").required(true).args(["id"])))]
pub struct Args {
    /// Row id of the queued prompt to delete (as surfaced by
    /// `agents queue list`).
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
            path_type: Path::AgentsQueueDelete,
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
