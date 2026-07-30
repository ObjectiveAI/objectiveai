//! `development viewer` — run the viewer app FROM SOURCE. A singleton
//! registration, not a per-plugin one: `set` points at an
//! `objectiveai-viewer` source directory and every subsequent
//! `viewer spawn` runs `pnpm exec tauri dev` there; `delete` restores
//! the installed binary; `get` reads the slot. Mode changes bounce a
//! RUNNING viewer immediately and never launch an absent one.

use crate::cli::command::CommandRequest;

pub mod delete;
pub mod get;
pub mod set;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Run the viewer from a source checkout (pnpm exec tauri dev).
    Set(set::Command),
    /// Show the current source registration, if any.
    Get(get::Command),
    /// Drop the registration, restoring the installed viewer.
    Delete(delete::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.development.viewer.Request")]
pub enum Request {
    #[schemars(title = "Set")]
    Set(set::Request),
    #[schemars(title = "SetRequestSchema")]
    SetRequestSchema(set::request_schema::Request),
    #[schemars(title = "SetResponseSchema")]
    SetResponseSchema(set::response_schema::Request),
    #[schemars(title = "Get")]
    Get(get::Request),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Request),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Request),
    #[schemars(title = "Delete")]
    Delete(delete::Request),
    #[schemars(title = "DeleteRequestSchema")]
    DeleteRequestSchema(delete::request_schema::Request),
    #[schemars(title = "DeleteResponseSchema")]
    DeleteResponseSchema(delete::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.viewer.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Set")]
    Set(set::Response),
    #[schemars(title = "SetRequestSchema")]
    SetRequestSchema(set::request_schema::Response),
    #[schemars(title = "SetResponseSchema")]
    SetResponseSchema(set::response_schema::Response),
    #[schemars(title = "Get")]
    Get(get::Response),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Response),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Response),
    #[schemars(title = "Delete")]
    Delete(delete::Response),
    #[schemars(title = "DeleteRequestSchema")]
    DeleteRequestSchema(delete::request_schema::Response),
    #[schemars(title = "DeleteResponseSchema")]
    DeleteResponseSchema(delete::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Set(v) => v.into_mcp(),
            ResponseItem::SetRequestSchema(v) => v.into_mcp(),
            ResponseItem::SetResponseSchema(v) => v.into_mcp(),
            ResponseItem::Get(v) => v.into_mcp(),
            ResponseItem::GetRequestSchema(v) => v.into_mcp(),
            ResponseItem::GetResponseSchema(v) => v.into_mcp(),
            ResponseItem::Delete(v) => v.into_mcp(),
            ResponseItem::DeleteRequestSchema(v) => v.into_mcp(),
            ResponseItem::DeleteResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Set(cmd) => match cmd.schema {
                None => Ok(Request::Set(set::Request::try_from(cmd.args)?)),
                Some(set::Schema::RequestSchema(args)) => Ok(
                    Request::SetRequestSchema(set::request_schema::Request::try_from(args)?),
                ),
                Some(set::Schema::ResponseSchema(args)) => Ok(
                    Request::SetResponseSchema(set::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) => Ok(
                    Request::GetRequestSchema(get::request_schema::Request::try_from(args)?),
                ),
                Some(get::Schema::ResponseSchema(args)) => Ok(
                    Request::GetResponseSchema(get::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Delete(cmd) => match cmd.schema {
                None => Ok(Request::Delete(delete::Request::try_from(cmd.args)?)),
                Some(delete::Schema::RequestSchema(args)) => Ok(
                    Request::DeleteRequestSchema(delete::request_schema::Request::try_from(args)?),
                ),
                Some(delete::Schema::ResponseSchema(args)) => Ok(
                    Request::DeleteResponseSchema(delete::response_schema::Request::try_from(args)?),
                ),
            },
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Set(inner) => inner.request_base(),
            Request::SetRequestSchema(inner) => inner.request_base(),
            Request::SetResponseSchema(inner) => inner.request_base(),
            Request::Get(inner) => inner.request_base(),
            Request::GetRequestSchema(inner) => inner.request_base(),
            Request::GetResponseSchema(inner) => inner.request_base(),
            Request::Delete(inner) => inner.request_base(),
            Request::DeleteRequestSchema(inner) => inner.request_base(),
            Request::DeleteResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Set(inner) => inner.request_base_mut(),
            Request::SetRequestSchema(inner) => inner.request_base_mut(),
            Request::SetResponseSchema(inner) => inner.request_base_mut(),
            Request::Get(inner) => inner.request_base_mut(),
            Request::GetRequestSchema(inner) => inner.request_base_mut(),
            Request::GetResponseSchema(inner) => inner.request_base_mut(),
            Request::Delete(inner) => inner.request_base_mut(),
            Request::DeleteRequestSchema(inner) => inner.request_base_mut(),
            Request::DeleteResponseSchema(inner) => inner.request_base_mut(),
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
        Request::Set(req) => {
            let value = set::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Set(value))))
        }
        Request::SetRequestSchema(req) => {
            let value = set::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::SetRequestSchema(value))))
        }
        Request::SetResponseSchema(req) => {
            let value = set::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::SetResponseSchema(value))))
        }
        Request::Get(req) => {
            let value = get::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Get(value))))
        }
        Request::GetRequestSchema(req) => {
            let value = get::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::GetRequestSchema(value))))
        }
        Request::GetResponseSchema(req) => {
            let value = get::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::GetResponseSchema(value))))
        }
        Request::Delete(req) => {
            let value = delete::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Delete(value))))
        }
        Request::DeleteRequestSchema(req) => {
            let value = delete::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::DeleteRequestSchema(value))))
        }
        Request::DeleteResponseSchema(req) => {
            let value = delete::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::DeleteResponseSchema(value))))
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
        Request::Set(req) => {
            let value = set::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::SetRequestSchema(req) => {
            let value = set::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::SetResponseSchema(req) => {
            let value = set::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Get(req) => {
            let value = get::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::GetRequestSchema(req) => {
            let value = get::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::GetResponseSchema(req) => {
            let value = get::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Delete(req) => {
            let value = delete::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DeleteRequestSchema(req) => {
            let value = delete::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DeleteResponseSchema(req) => {
            let value = delete::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub enum ListenerExecution {
    Set(set::ListenerExecution),
    SetRequestSchema(set::request_schema::ListenerExecution),
    SetResponseSchema(set::response_schema::ListenerExecution),
    Get(get::ListenerExecution),
    GetRequestSchema(get::request_schema::ListenerExecution),
    GetResponseSchema(get::response_schema::ListenerExecution),
    Delete(delete::ListenerExecution),
    DeleteRequestSchema(delete::request_schema::ListenerExecution),
    DeleteResponseSchema(delete::response_schema::ListenerExecution),
}
