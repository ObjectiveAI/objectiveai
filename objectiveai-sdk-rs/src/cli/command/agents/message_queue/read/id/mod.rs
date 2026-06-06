//! `agents message-queue read id` Ã¢â‚¬â€ fetch one piece of queued content by
//! its `prompt_contents.id`.
//!
//! `agents message-queue list` emits a `content: ResponseContent` field on
//! each item (`One(i64)` or `Many(Vec<i64>)`) Ã¢â‚¬â€ the same shape
//! `RichContent` decomposes to. This leaf takes one such id and
//! returns the typed payload directly as a
//! [`crate::agent::completions::message::RichContentPart`], so the
//! wire form for queue content is bit-identical to a rich-content
//! part of the same kind. No fork between queue media and message
//! media at the type level.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.read.id.Request")]
pub struct Request {
    pub path_type: Path,
    pub id: i64,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.read.id.Path")]
pub enum Path {
    #[serde(rename = "agents/message-queue/read/id")]
    AgentsQueueReadId,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "message-queue".to_string(),
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

/// The typed payload of one `prompt_contents.id`. Aliased directly
/// to the SDK [`RichContentPart`] so the wire shape matches
/// rich-content parts exactly Ã¢â‚¬â€ tagged by `type` with `text`,
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

// `RichContentPart` already implements `CommandResponse` via the
// canonical impl in `cli::command::command_response.rs` Ã¢â‚¬â€ no leaf-
// local impl needed (and adding one would conflict).

pub mod request_schema;

pub mod response_schema;
