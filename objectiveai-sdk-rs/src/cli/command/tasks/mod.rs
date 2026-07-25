//! `tasks` — durable scheduled commands. A task is an ObjectiveAI
//! command plus a schedule; the daemon's resident scheduler fires it
//! with the identity it was CREATED with (agent arguments + plugin
//! trio) and marks the run with the daemon-authored `task` identity
//! flag. Leaves:
//!
//! - `create` — schedule a command (`--delay-secs`, optional
//!   `--repeat` / `--repeat-count`).
//! - `list` — every task with its run/error counters and last result.
//! - `delete` — remove a task by id.

use crate::cli::command::CommandRequest;

pub mod create;
pub mod delete;
pub mod list;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Schedule an ObjectiveAI command.
    Create(create::Command),
    /// List every task with its run counters and last result.
    List(list::Command),
    /// Remove a task by id.
    Delete(delete::Command),
}

// No `PartialEq`: `create::Request` embeds the root request enum,
// which doesn't derive it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.tasks.Request")]
pub enum Request {
    #[schemars(title = "Create")]
    Create(create::Request),
    #[schemars(title = "CreateRequestSchema")]
    CreateRequestSchema(create::request_schema::Request),
    #[schemars(title = "CreateResponseSchema")]
    CreateResponseSchema(create::response_schema::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
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
#[schemars(rename = "cli.command.tasks.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Create")]
    Create(create::Response),
    #[schemars(title = "CreateRequestSchema")]
    CreateRequestSchema(create::request_schema::Response),
    #[schemars(title = "CreateResponseSchema")]
    CreateResponseSchema(create::response_schema::Response),
    #[schemars(title = "List")]
    List(list::ResponseItem),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
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
            ResponseItem::Create(v) => v.into_mcp(),
            ResponseItem::CreateRequestSchema(v) => v.into_mcp(),
            ResponseItem::CreateResponseSchema(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
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
            Command::Create(cmd) => match cmd.schema {
                None => Ok(Request::Create(create::Request::try_from(cmd.args)?)),
                Some(create::Schema::RequestSchema(args)) => Ok(
                    Request::CreateRequestSchema(create::request_schema::Request::try_from(args)?),
                ),
                Some(create::Schema::ResponseSchema(args)) => Ok(
                    Request::CreateResponseSchema(create::response_schema::Request::try_from(args)?),
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
            Request::Create(inner) => inner.request_base(),
            Request::CreateRequestSchema(inner) => inner.request_base(),
            Request::CreateResponseSchema(inner) => inner.request_base(),
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
            Request::Delete(inner) => inner.request_base(),
            Request::DeleteRequestSchema(inner) => inner.request_base(),
            Request::DeleteResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Create(inner) => inner.request_base_mut(),
            Request::CreateRequestSchema(inner) => inner.request_base_mut(),
            Request::CreateResponseSchema(inner) => inner.request_base_mut(),
            Request::List(inner) => inner.request_base_mut(),
            Request::ListRequestSchema(inner) => inner.request_base_mut(),
            Request::ListResponseSchema(inner) => inner.request_base_mut(),
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
    use futures::StreamExt;
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
        Request::Create(req) => {
            let value = create::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Create(value))))
        }
        Request::CreateRequestSchema(req) => {
            let value = create::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::CreateRequestSchema(value))))
        }
        Request::CreateResponseSchema(req) => {
            let value = create::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::CreateResponseSchema(value))))
        }
        Request::List(req) => {
            let inner = list::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::ListRequestSchema(value))))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::ListResponseSchema(value))))
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
        Request::Create(req) => {
            let value = create::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::CreateRequestSchema(req) => {
            let value = create::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::CreateResponseSchema(req) => {
            let value = create::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::List(req) => {
            let inner = list::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute_transform(executor, req, transform, identity).await?;
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
    Create(create::ListenerExecution),
    CreateRequestSchema(create::request_schema::ListenerExecution),
    CreateResponseSchema(create::response_schema::ListenerExecution),
    List(list::ListenerExecution),
    ListRequestSchema(list::request_schema::ListenerExecution),
    ListResponseSchema(list::response_schema::ListenerExecution),
    Delete(delete::ListenerExecution),
    DeleteRequestSchema(delete::request_schema::ListenerExecution),
    DeleteResponseSchema(delete::response_schema::ListenerExecution),
}
