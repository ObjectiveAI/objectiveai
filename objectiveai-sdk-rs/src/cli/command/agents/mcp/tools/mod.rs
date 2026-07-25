//! `agents mcp tools` — call the per-`response_id` MCP listener's tool
//! ops over its local socket. Subcommands:
//!
//! - `call --response-id <id> --params <json>` — `tools/call`.
//! - `list --response-id <id> --params <json>` — `tools/list`.
//!
//! `--params` is the strongly typed MCP request body supplied as a
//! JSON string; the command returns the MCP result.

use crate::cli::command::CommandRequest;

pub mod call;
pub mod list;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Invoke `tools/call` against the agent's MCP aggregation.
    Call(call::Command),
    /// Invoke `tools/list` against the agent's MCP aggregation.
    List(list::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.mcp.tools.Request")]
pub enum Request {
    #[schemars(title = "Call")]
    Call(call::Request),
    #[schemars(title = "CallRequestSchema")]
    CallRequestSchema(call::request_schema::Request),
    #[schemars(title = "CallResponseSchema")]
    CallResponseSchema(call::response_schema::Request),
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
#[schemars(rename = "cli.command.agents.mcp.tools.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Call")]
    Call(call::Response),
    #[schemars(title = "CallRequestSchema")]
    CallRequestSchema(call::request_schema::Response),
    #[schemars(title = "CallResponseSchema")]
    CallResponseSchema(call::response_schema::Response),
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
            ResponseItem::Call(v) => v.into_mcp(),
            ResponseItem::CallRequestSchema(v) => v.into_mcp(),
            ResponseItem::CallResponseSchema(v) => v.into_mcp(),
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
            Command::Call(cmd) => match cmd.schema {
                None => Ok(Request::Call(call::Request::try_from(cmd.args)?)),
                Some(call::Schema::RequestSchema(args)) => Ok(
                    Request::CallRequestSchema(call::request_schema::Request::try_from(args)?),
                ),
                Some(call::Schema::ResponseSchema(args)) => Ok(
                    Request::CallResponseSchema(call::response_schema::Request::try_from(args)?),
                ),
            },
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
            Request::Call(inner) => inner.request_base(),
            Request::CallRequestSchema(inner) => inner.request_base(),
            Request::CallResponseSchema(inner) => inner.request_base(),
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Call(inner) => inner.request_base_mut(),
            Request::CallRequestSchema(inner) => inner.request_base_mut(),
            Request::CallResponseSchema(inner) => inner.request_base_mut(),
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
        Request::Call(req) => {
            let value = call::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Call(value),
            )))
        }
        Request::CallRequestSchema(req) => {
            let value =
                call::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::CallRequestSchema(value),
            )))
        }
        Request::CallResponseSchema(req) => {
            let value =
                call::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::CallResponseSchema(value),
            )))
        }
        Request::List(req) => {
            let value = list::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::List(value),
            )))
        }
        Request::ListRequestSchema(req) => {
            let value =
                list::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListRequestSchema(value),
            )))
        }
        Request::ListResponseSchema(req) => {
            let value =
                list::response_schema::execute(executor, req, identity).await?;
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
        Request::Call(req) => {
            let value = call::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::CallRequestSchema(req) => {
            let value =
                call::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::CallResponseSchema(req) => {
            let value =
                call::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
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
/// its `ListenerExecution`. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Call(call::ListenerExecution),
    CallRequestSchema(call::request_schema::ListenerExecution),
    CallResponseSchema(call::response_schema::ListenerExecution),
    List(list::ListenerExecution),
    ListRequestSchema(list::request_schema::ListenerExecution),
    ListResponseSchema(list::response_schema::ListenerExecution),
}
