//! `agents queue read id` — fetch one piece of queued content by
//! its `prompt_contents.id`.
//!
//! `agents queue list` emits a `content: ResponseContent` field on
//! each item (`One(i64)` or `Many(Vec<i64>)`) — the same shape
//! `RichContent` decomposes to. This leaf takes one such id and
//! returns the typed payload directly as a
//! [`crate::agent::completions::message::RichContentPart`], so the
//! wire form for queue content is bit-identical to a rich-content
//! part of the same kind. No fork between queue media and message
//! media at the type level.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.open.Request")]
pub struct Request {
    pub path_type: Path,
    pub id: i64,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.open.Path")]
pub enum Path {
    #[serde(rename = "agents/queue/open")]
    AgentsQueueOpen,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The typed payload of one `prompt_contents.id`. Aliased directly
/// to the SDK [`RichContentPart`] so the wire shape matches
/// rich-content parts exactly — tagged by `type` with `text`,
/// `image_url`, `input_audio`, `input_video`, `video_url`, or
/// `file`. Queue content production today never emits the
/// `input_video` variant (the walker stores both `InputVideo` and
/// `VideoUrl` parts as `prompt_videos` rows reading back as
/// `video_url`), but the type stays unconstrained for forward
/// compatibility.
///
/// [`RichContentPart`]: crate::agent::completions::message::RichContentPart
pub type Response = crate::agent::completions::message::RichContentPart;

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("id_required").required(true).args(["id"])))]
pub struct Args {
    /// `prompt_contents.id` of the content row to fetch.
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
            path_type: Path::AgentsQueueOpen,
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

// `RichContentPart` already implements `CommandResponse` via the
// canonical impl in `cli::command::command_response.rs` — no leaf-
// local impl needed (and adding one would conflict).

pub mod request_schema;

pub mod response_schema;
