//! `agents queue` — deferred prompts queue. Top-level subcommands:
//!
//! - `open --id <id>` — fetch one piece of queued content by its
//!   `prompt_contents.id`. The wire shape mirrors `RichContentPart`
//!   (tagged by `type`).
//! - `list --pending` — stream the queued prompts pending delivery
//!   under the resolved targets.
//! - `delete --id <id>` — remove one queued prompt by id.
//! - `deliver` — wake every queue-pending strict descendant of the
//!   caller (try-lock each AIH; spawn the idle ones with empty
//!   messages so they drain their own queues).
//!
//! Enqueue is no longer a CLI verb here — use `agents message`
//! instead; it handles persistence under the hood.

use crate::cli::command::CommandRequest;

pub mod delete;
pub mod deliver;
pub mod list;
pub mod open;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Delete one queued prompt by id.
    Delete(delete::Command),
    /// Wake every queue-pending descendant agent of the caller.
    Deliver(deliver::Command),
    /// List queued prompts pending delivery under the targets.
    List(list::Command),
    /// Fetch one piece of queued content by `prompt_contents.id`.
    Open(open::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.queue.Request")]
pub enum Request {
    #[schemars(title = "Delete")]
    Delete(delete::Request),
    #[schemars(title = "DeleteRequestSchema")]
    DeleteRequestSchema(delete::request_schema::Request),
    #[schemars(title = "DeleteResponseSchema")]
    DeleteResponseSchema(delete::response_schema::Request),
    #[schemars(title = "Deliver")]
    Deliver(deliver::Request),
    #[schemars(title = "DeliverRequestSchema")]
    DeliverRequestSchema(deliver::request_schema::Request),
    #[schemars(title = "DeliverResponseSchema")]
    DeliverResponseSchema(deliver::response_schema::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
    #[schemars(title = "Open")]
    Open(open::Request),
    #[schemars(title = "OpenRequestSchema")]
    OpenRequestSchema(open::request_schema::Request),
    #[schemars(title = "OpenResponseSchema")]
    OpenResponseSchema(open::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Delete")]
    Delete(delete::Response),
    #[schemars(title = "DeleteRequestSchema")]
    DeleteRequestSchema(delete::request_schema::Response),
    #[schemars(title = "DeleteResponseSchema")]
    DeleteResponseSchema(delete::response_schema::Response),
    #[schemars(title = "Deliver")]
    Deliver(deliver::ResponseItem),
    #[schemars(title = "DeliverRequestSchema")]
    DeliverRequestSchema(deliver::request_schema::Response),
    #[schemars(title = "DeliverResponseSchema")]
    DeliverResponseSchema(deliver::response_schema::Response),
    #[schemars(title = "List")]
    List(list::ResponseItem),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
    #[schemars(title = "Open")]
    Open(open::Response),
    #[schemars(title = "OpenRequestSchema")]
    OpenRequestSchema(open::request_schema::Response),
    #[schemars(title = "OpenResponseSchema")]
    OpenResponseSchema(open::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Delete(v) => v.into_mcp(),
            ResponseItem::DeleteRequestSchema(v) => v.into_mcp(),
            ResponseItem::DeleteResponseSchema(v) => v.into_mcp(),
            ResponseItem::Deliver(v) => v.into_mcp(),
            ResponseItem::DeliverRequestSchema(v) => v.into_mcp(),
            ResponseItem::DeliverResponseSchema(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
            ResponseItem::Open(v) => v.into_mcp(),
            ResponseItem::OpenRequestSchema(v) => v.into_mcp(),
            ResponseItem::OpenResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Delete(cmd) => match cmd.schema {
                None => Ok(Request::Delete(delete::Request::try_from(cmd.args)?)),
                Some(delete::Schema::RequestSchema(args)) => Ok(
                    Request::DeleteRequestSchema(delete::request_schema::Request::try_from(args)?),
                ),
                Some(delete::Schema::ResponseSchema(args)) => Ok(
                    Request::DeleteResponseSchema(delete::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Deliver(cmd) => match cmd.schema {
                None => Ok(Request::Deliver(deliver::Request::try_from(cmd.args)?)),
                Some(deliver::Schema::RequestSchema(args)) => Ok(
                    Request::DeliverRequestSchema(deliver::request_schema::Request::try_from(args)?),
                ),
                Some(deliver::Schema::ResponseSchema(args)) => Ok(
                    Request::DeliverResponseSchema(deliver::response_schema::Request::try_from(args)?),
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
            Command::Open(cmd) => match cmd.schema {
                None => Ok(Request::Open(open::Request::try_from(cmd.args)?)),
                Some(open::Schema::RequestSchema(args)) => Ok(
                    Request::OpenRequestSchema(open::request_schema::Request::try_from(args)?),
                ),
                Some(open::Schema::ResponseSchema(args)) => Ok(
                    Request::OpenResponseSchema(open::response_schema::Request::try_from(args)?),
                ),
            },
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Delete(inner) => inner.request_base(),
            Request::DeleteRequestSchema(inner) => inner.request_base(),
            Request::DeleteResponseSchema(inner) => inner.request_base(),
            Request::Deliver(inner) => inner.request_base(),
            Request::DeliverRequestSchema(inner) => inner.request_base(),
            Request::DeliverResponseSchema(inner) => inner.request_base(),
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
            Request::Open(inner) => inner.request_base(),
            Request::OpenRequestSchema(inner) => inner.request_base(),
            Request::OpenResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Delete(inner) => inner.request_base_mut(),
            Request::DeleteRequestSchema(inner) => inner.request_base_mut(),
            Request::DeleteResponseSchema(inner) => inner.request_base_mut(),
            Request::Deliver(inner) => inner.request_base_mut(),
            Request::DeliverRequestSchema(inner) => inner.request_base_mut(),
            Request::DeliverResponseSchema(inner) => inner.request_base_mut(),
            Request::List(inner) => inner.request_base_mut(),
            Request::ListRequestSchema(inner) => inner.request_base_mut(),
            Request::ListResponseSchema(inner) => inner.request_base_mut(),
            Request::Open(inner) => inner.request_base_mut(),
            Request::OpenRequestSchema(inner) => inner.request_base_mut(),
            Request::OpenResponseSchema(inner) => inner.request_base_mut(),
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
        Request::Delete(req) => {
            let value = delete::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Delete(value),
            )))
        }
        Request::DeleteRequestSchema(req) => {
            let value =
                delete::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::DeleteRequestSchema(value),
            )))
        }
        Request::DeleteResponseSchema(req) => {
            let value =
                delete::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::DeleteResponseSchema(value),
            )))
        }
        Request::Deliver(req) => {
            let inner = deliver::execute(executor, req, agent_arguments).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Deliver)))
        }
        Request::DeliverRequestSchema(req) => {
            let value =
                deliver::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::DeliverRequestSchema(value),
            )))
        }
        Request::DeliverResponseSchema(req) => {
            let value =
                deliver::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::DeliverResponseSchema(value),
            )))
        }
        Request::List(req) => {
            let inner = list::execute(executor, req, agent_arguments).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value =
                list::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListRequestSchema(value),
            )))
        }
        Request::ListResponseSchema(req) => {
            let value =
                list::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListResponseSchema(value),
            )))
        }
        Request::Open(req) => {
            let value = open::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Open(value),
            )))
        }
        Request::OpenRequestSchema(req) => {
            let value =
                open::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::OpenRequestSchema(value),
            )))
        }
        Request::OpenResponseSchema(req) => {
            let value =
                open::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::OpenResponseSchema(value),
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
        Request::Delete(req) => {
            let value = delete::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DeleteRequestSchema(req) => {
            let value =
                delete::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DeleteResponseSchema(req) => {
            let value =
                delete::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Deliver(req) => {
            let inner = deliver::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::DeliverRequestSchema(req) => {
            let value =
                deliver::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DeliverResponseSchema(req) => {
            let value =
                deliver::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::List(req) => {
            let inner = list::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::ListRequestSchema(req) => {
            let value =
                list::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListResponseSchema(req) => {
            let value =
                list::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Open(req) => {
            let value = open::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::OpenRequestSchema(req) => {
            let value =
                open::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::OpenResponseSchema(req) => {
            let value =
                open::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}
