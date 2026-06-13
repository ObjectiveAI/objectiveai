//! `agents queue` — deferred prompts queue. Three top-level
//! subcommands:
//!
//! - `delete` — remove one queued prompt by id.
//! - `deliver` — wake every queue-pending strict descendant of the
//!   caller (try-lock each AIH; spawn the idle ones with empty
//!   messages so they drain their own queues).
//! - `read` (nested) — sub-tier whose only leaf today is `id`,
//!   which fetches one piece of queued content by its
//!   `prompt_contents.id`. The wire shape mirrors `RichContentPart`
//!   (tagged by `type`).
//!
//! Enqueue is no longer a CLI verb here — use `agents message`
//! instead; it handles persistence under the hood. Drain is also
//! gone — the API consumes queue rows directly via the WS reverse-
//! attach `read_message_queue` / `clear_message_queue` server
//! requests once a matching hierarchy comes online.

use crate::cli::command::CommandRequest;

pub mod delete;
pub mod deliver;
pub mod read;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Delete one queued prompt by id.
    Delete(delete::Command),
    /// Wake every queue-pending descendant agent of the caller.
    Deliver(deliver::Command),
    /// Read queued content — `read id <id>` for a single content
    /// piece, `read pending [parent]` for the list of queued
    /// prompts under a parent.
    Read(ReadCommand),
}

/// Intermediate clap level for the `read` sub-tier. Splitting it
/// into its own wrapper (rather than a fattened `ReadId` variant on
/// [`Command`]) gives the CLI surface `agents queue read id <num>`
/// to match the user's invocation style and keeps the door open for
/// additional `read <…>` leaves later.
#[derive(clap::Args)]
pub struct ReadCommand {
    #[command(subcommand)]
    pub sub: read::Command,
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
    #[schemars(title = "Read")]
    Read(read::Request),
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
    #[schemars(title = "Read")]
    Read(read::ResponseItem),
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
            ResponseItem::Read(v) => v.into_mcp(),
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
            Command::Read(rc) => Ok(Request::Read(read::Request::try_from(rc.sub)?)),
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Delete(inner) => inner.into_command(),
            Request::DeleteRequestSchema(inner) => inner.into_command(),
            Request::DeleteResponseSchema(inner) => inner.into_command(),
            Request::Deliver(inner) => inner.into_command(),
            Request::DeliverRequestSchema(inner) => inner.into_command(),
            Request::DeliverResponseSchema(inner) => inner.into_command(),
            Request::Read(inner) => inner.into_command(),
        }
    }

    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Delete(inner) => inner.request_base(),
            Request::DeleteRequestSchema(inner) => inner.request_base(),
            Request::DeleteResponseSchema(inner) => inner.request_base(),
            Request::Deliver(inner) => inner.request_base(),
            Request::DeliverRequestSchema(inner) => inner.request_base(),
            Request::DeliverResponseSchema(inner) => inner.request_base(),
            Request::Read(inner) => inner.request_base(),
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
            Request::Read(inner) => inner.request_base_mut(),
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
        Request::Read(req) => {
            let inner = read::execute(executor, req, agent_arguments).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Read)))
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
        Request::Read(req) => {
            let inner = read::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(inner)
        }
    };
    Ok(stream)
}
