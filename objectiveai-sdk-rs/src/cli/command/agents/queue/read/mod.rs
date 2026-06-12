//! `agents queue read` sub-tier. Two leaves:
//!
//! * `id <id>` — fetch one piece of queued content by
//!   `prompt_contents.id`.
//! * `pending [parent]` — stream queued prompts under `parent` (or
//!   the cli's own position when omitted).

use crate::cli::command::CommandRequest;

pub mod id;
pub mod pending;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Fetch one piece of queued content by `prompt_contents.id`.
    Id(id::Command),
    /// Stream queued prompts pending delivery under a parent.
    Pending(pending::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.queue.read.Request")]
pub enum Request {
    #[schemars(title = "Id")]
    Id(id::Request),
    #[schemars(title = "IdRequestSchema")]
    IdRequestSchema(id::request_schema::Request),
    #[schemars(title = "IdResponseSchema")]
    IdResponseSchema(id::response_schema::Request),
    #[schemars(title = "Pending")]
    Pending(pending::Request),
    #[schemars(title = "PendingRequestSchema")]
    PendingRequestSchema(pending::request_schema::Request),
    #[schemars(title = "PendingResponseSchema")]
    PendingResponseSchema(pending::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.read.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Id")]
    Id(id::Response),
    #[schemars(title = "IdRequestSchema")]
    IdRequestSchema(id::request_schema::Response),
    #[schemars(title = "IdResponseSchema")]
    IdResponseSchema(id::response_schema::Response),
    #[schemars(title = "Pending")]
    Pending(pending::ResponseItem),
    #[schemars(title = "PendingRequestSchema")]
    PendingRequestSchema(pending::request_schema::Response),
    #[schemars(title = "PendingResponseSchema")]
    PendingResponseSchema(pending::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Id(v) => v.into_mcp(),
            ResponseItem::IdRequestSchema(v) => v.into_mcp(),
            ResponseItem::IdResponseSchema(v) => v.into_mcp(),
            ResponseItem::Pending(v) => v.into_mcp(),
            ResponseItem::PendingRequestSchema(v) => v.into_mcp(),
            ResponseItem::PendingResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Id(cmd) => match cmd.schema {
                None => Ok(Request::Id(id::Request::try_from(cmd.args)?)),
                Some(id::Schema::RequestSchema(args)) => Ok(
                    Request::IdRequestSchema(id::request_schema::Request::try_from(args)?),
                ),
                Some(id::Schema::ResponseSchema(args)) => Ok(
                    Request::IdResponseSchema(id::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Pending(cmd) => match cmd.schema {
                None => Ok(Request::Pending(pending::Request::try_from(cmd.args)?)),
                Some(pending::Schema::RequestSchema(args)) => Ok(Request::PendingRequestSchema(
                    pending::request_schema::Request::try_from(args)?,
                )),
                Some(pending::Schema::ResponseSchema(args)) => Ok(Request::PendingResponseSchema(
                    pending::response_schema::Request::try_from(args)?,
                )),
            },
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Id(inner) => inner.into_command(),
            Request::IdRequestSchema(inner) => inner.into_command(),
            Request::IdResponseSchema(inner) => inner.into_command(),
            Request::Pending(inner) => inner.into_command(),
            Request::PendingRequestSchema(inner) => inner.into_command(),
            Request::PendingResponseSchema(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
        Request::Id(req) => {
            let value = id::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Id(value))))
        }
        Request::IdRequestSchema(req) => {
            let value =
                id::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::IdRequestSchema(value),
            )))
        }
        Request::IdResponseSchema(req) => {
            let value =
                id::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::IdResponseSchema(value),
            )))
        }
        Request::Pending(req) => {
            let inner = pending::execute(executor, req, agent_arguments).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Pending)))
        }
        Request::PendingRequestSchema(req) => {
            let value =
                pending::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::PendingRequestSchema(value),
            )))
        }
        Request::PendingResponseSchema(req) => {
            let value =
                pending::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::PendingResponseSchema(value),
            )))
        }
    };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    transform: crate::cli::command::Transform,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>,
    > = match request {
        Request::Id(req) => {
            let value = id::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::IdRequestSchema(req) => {
            let value =
                id::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::IdResponseSchema(req) => {
            let value =
                id::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Pending(req) => {
            let inner = pending::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::PendingRequestSchema(req) => {
            let value =
                pending::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::PendingResponseSchema(req) => {
            let value =
                pending::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}
