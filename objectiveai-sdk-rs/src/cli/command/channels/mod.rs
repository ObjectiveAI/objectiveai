//! `channels` — duplex, capability-gated channels between a publisher
//! (a trusted `/execute` caller) and an owner (a `/channels` SSE
//! client). Leaves:
//!
//! - `publish` — offer a channel and block until accepted.
//! - `logs …` — the per-channel append-only message log.

use crate::cli::command::CommandRequest;

pub mod logs;
pub mod publish;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Offer a channel and block until the first client accepts.
    Publish(publish::Command),
    /// The per-channel message log.
    Logs {
        #[command(subcommand)]
        command: logs::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.channels.Request")]
pub enum Request {
    #[schemars(title = "Publish")]
    Publish(publish::Request),
    #[schemars(title = "PublishRequestSchema")]
    PublishRequestSchema(publish::request_schema::Request),
    #[schemars(title = "PublishResponseSchema")]
    PublishResponseSchema(publish::response_schema::Request),
    #[schemars(title = "Logs")]
    Logs(logs::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Publish")]
    Publish(publish::Response),
    #[schemars(title = "PublishRequestSchema")]
    PublishRequestSchema(publish::request_schema::Response),
    #[schemars(title = "PublishResponseSchema")]
    PublishResponseSchema(publish::response_schema::Response),
    #[schemars(title = "Logs")]
    Logs(logs::ResponseItem),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Publish(v) => v.into_mcp(),
            ResponseItem::PublishRequestSchema(v) => v.into_mcp(),
            ResponseItem::PublishResponseSchema(v) => v.into_mcp(),
            ResponseItem::Logs(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Publish(cmd) => match cmd.schema {
                None => Ok(Request::Publish(publish::Request::try_from(cmd.args)?)),
                Some(publish::Schema::RequestSchema(args)) => Ok(
                    Request::PublishRequestSchema(publish::request_schema::Request::try_from(args)?),
                ),
                Some(publish::Schema::ResponseSchema(args)) => Ok(
                    Request::PublishResponseSchema(publish::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Logs { command } => Ok(Request::Logs(logs::Request::try_from(command)?)),
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Publish(inner) => inner.request_base(),
            Request::PublishRequestSchema(inner) => inner.request_base(),
            Request::PublishResponseSchema(inner) => inner.request_base(),
            Request::Logs(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Publish(inner) => inner.request_base_mut(),
            Request::PublishRequestSchema(inner) => inner.request_base_mut(),
            Request::PublishResponseSchema(inner) => inner.request_base_mut(),
            Request::Logs(inner) => inner.request_base_mut(),
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
        Request::Publish(req) => {
            let value = publish::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Publish(value))))
        }
        Request::PublishRequestSchema(req) => {
            let value = publish::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::PublishRequestSchema(value))))
        }
        Request::PublishResponseSchema(req) => {
            let value = publish::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::PublishResponseSchema(value))))
        }
        Request::Logs(req) => {
            let inner = logs::execute(executor, req, agent_arguments).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Logs)))
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
        Request::Publish(req) => {
            let value = publish::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::PublishRequestSchema(req) => {
            let value = publish::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::PublishResponseSchema(req) => {
            let value = publish::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Logs(req) => {
            let inner = logs::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(inner)
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Publish(publish::ListenerExecution),
    PublishRequestSchema(publish::request_schema::ListenerExecution),
    PublishResponseSchema(publish::response_schema::ListenerExecution),
    Logs(logs::ListenerExecution),
}
