//! `agents mcp servers` — list the upstream MCP servers connected for a live
//! agent's `response_id`. Subcommands:
//!
//! - `list --response-id <id>` — `servers/list` (proxy-local aggregate).

use crate::cli::command::CommandRequest;

pub mod list;

#[derive(clap::Subcommand)]
pub enum Command {
    /// List the agent's connected MCP servers and their metadata.
    List(list::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.mcp.servers.Request")]
pub enum Request {
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.mcp.servers.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "List")]
    List(list::Response),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) => Ok(
                    Request::ListRequestSchema(list::request_schema::Request::try_from(args)?),
                ),
                Some(list::Schema::ResponseSchema(args)) => Ok(
                    Request::ListResponseSchema(list::response_schema::Request::try_from(args)?),
                ),
            },
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::List(inner) => inner.request_base_mut(),
            Request::ListRequestSchema(inner) => inner.request_base_mut(),
            Request::ListResponseSchema(inner) => inner.request_base_mut(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    identity: Option<&crate::identity::Identity>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
        Request::List(req) => {
            let value = list::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::List(value),
            )))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListRequestSchema(value),
            )))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListResponseSchema(value),
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
    identity: Option<&crate::identity::Identity>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>,
    > = match request {
        Request::List(req) => {
            let value = list::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListRequestSchema(req) => {
            let value =
                list::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListResponseSchema(req) => {
            let value =
                list::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub enum ListenerExecution {
    List(list::ListenerExecution),
    ListRequestSchema(list::request_schema::ListenerExecution),
    ListResponseSchema(list::response_schema::ListenerExecution),
}
