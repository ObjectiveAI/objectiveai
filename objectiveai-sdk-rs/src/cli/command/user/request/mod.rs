//! `user request` — broadcast an outbound request to every connected
//! user stream (`GET /user`) and BLOCK until the first ACCEPTED reply
//! (or the base `--timeout` elapses).
//!
//! The daemon holds the request PENDING even with zero connected
//! streams — a user surface that connects later receives it on
//! replay. An optional `--validate-python` snippet gates replies:
//! it runs with the full reply (`{"identity": …, "reply": …}`) as its
//! `input` and must end in a trailing expression evaluating `True`
//! for the reply to be accepted; anything else (False, no output, an
//! exception) rejects it and the request stays pending. Without the
//! base `--timeout` the wait is UNCAPPED.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.user.request.Request")]
pub struct Request {
    pub path_type: Path,
    /// Caller-chosen discriminator (e.g. `"AskUserQuestion"`) — how a
    /// user surface decides what UI the `details` drive.
    pub key: String,
    /// Arbitrary request payload, opaque to the daemon.
    pub details: serde_json::Value,
    /// Optional reply validator: python whose `input` is the full
    /// reply and whose trailing expression must evaluate `True` to
    /// accept it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub validate_python: Option<String>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.user.request.Path")]
pub enum Path {
    #[serde(rename = "user/request")]
    UserRequest,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The winning reply.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.user.request.Response")]
pub struct Response {
    /// The replier's identity (from its `X-OBJECTIVEAI-*` headers).
    pub identity: crate::cli::command::AgentArguments,
    /// The accepted reply payload.
    pub reply: serde_json::Value,
}

#[derive(clap::Args)]
pub struct Args {
    /// Caller-chosen discriminator (e.g. "AskUserQuestion").
    #[arg(long)]
    pub key: String,
    /// The request payload as inline JSON.
    #[arg(long)]
    pub details: String,
    /// Optional reply validator: python receiving the full reply as
    /// `input`; its trailing expression must evaluate True to accept.
    #[arg(long)]
    pub validate_python: Option<String>,
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
        let mut de = serde_json::Deserializer::from_str(&args.details);
        let details = serde_path_to_error::deserialize(&mut de)
            .map_err(|e| crate::cli::command::FromArgsError::json("details", e))?;
        Ok(Self {
            path_type: Path::UserRequest,
            key: args.key,
            details,
            validate_python: args.validate_python,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
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

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    _transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
    let resp: Response = executor.execute_one(request, agent_arguments).await?;
    Ok(serde_json::to_value(resp).expect("Response serializes"))
}

/// One `/listen` broadcast run of `user request`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
