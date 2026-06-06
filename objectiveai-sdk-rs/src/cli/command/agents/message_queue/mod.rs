//! `agents message-queue` Ã¢â‚¬â€ deferred prompts queue. Two top-level subcommands:
//!
//! - `add` Ã¢â‚¬â€ write-only enqueue. Stores a prompt in `tags.sqlite`
//!   (the `prompts` table) against either a resolved
//!   `agent_instance_hierarchy` (Direct mode) or a literal
//!   `agent_tag` (Tag mode Ã¢â‚¬â€ no resolution at enqueue time).
//! - `list` Ã¢â‚¬â€ streaming inspection. Both Direct and Tag rows are
//!   filtered to direct children of a parent (Tag rows resolve
//!   their parent via the joined `tags` table). Each tag-row item
//!   carries the joined 3-state status and the resolved prompt
//!   body as `Vec<ResponseQueueMessage>` (id-referenced into the
//!   per-kind content tables).
//! - `read` (nested) Ã¢â‚¬â€ sub-tier whose only leaf today is `id`,
//!   which fetches one piece of queued content by its
//!   `prompt_contents.id`. The wire shape mirrors `RichContentPart`
//!   (tagged by `type`).
//!
//! Dequeue / delete leaves will land in follow-up passes.

use crate::cli::command::CommandRequest;

pub mod add;
pub mod deliver;
pub mod delete;
pub mod read;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Enqueue a prompt against a target agent.
    Add(add::Command),
    /// Delete one queued prompt by id.
    Delete(delete::Command),
    /// Fan out `agents message` against every BOUND target with
    /// pending queue rows under a parent hierarchy, in parallel.
    Deliver(deliver::Command),
    /// Read queued content — `read id <id>` for a single content
    /// piece, `read pending [parent]` for the list of queued
    /// prompts under a parent.
    Read(ReadCommand),
}

/// Intermediate clap level for the `read` sub-tier Ã¢â‚¬â€ its only
/// subcommand today is `id`. Splitting it into its own wrapper
/// (rather than a fattened `ReadId` variant on [`Command`]) gives
/// the CLI surface `agents message-queue read id <num>` to match the user's
/// invocation style and keeps the door open for additional
/// `read <Ã¢â‚¬Â¦>` leaves later.
#[derive(clap::Args)]
pub struct ReadCommand {
    #[command(subcommand)]
    pub sub: read::Command,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.message_queue.Request")]
pub enum Request {
    #[schemars(title = "Add")]
    Add(add::Request),
    #[schemars(title = "AddRequestSchema")]
    AddRequestSchema(add::request_schema::Request),
    #[schemars(title = "AddResponseSchema")]
    AddResponseSchema(add::response_schema::Request),
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
#[schemars(rename = "cli.command.agents.message_queue.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Add")]
    Add(add::Response),
    #[schemars(title = "AddRequestSchema")]
    AddRequestSchema(add::request_schema::Response),
    #[schemars(title = "AddResponseSchema")]
    AddResponseSchema(add::response_schema::Response),
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
            ResponseItem::Add(v) => v.into_mcp(),
            ResponseItem::AddRequestSchema(v) => v.into_mcp(),
            ResponseItem::AddResponseSchema(v) => v.into_mcp(),
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
            Command::Add(cmd) => match cmd.schema {
                None => Ok(Request::Add(add::Request::try_from(cmd.args)?)),
                Some(add::Schema::RequestSchema(args)) => Ok(
                    Request::AddRequestSchema(add::request_schema::Request::try_from(args)?),
                ),
                Some(add::Schema::ResponseSchema(args)) => Ok(
                    Request::AddResponseSchema(add::response_schema::Request::try_from(args)?),
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
            Request::Add(inner) => inner.into_command(),
            Request::AddRequestSchema(inner) => inner.into_command(),
            Request::AddResponseSchema(inner) => inner.into_command(),
            Request::Delete(inner) => inner.into_command(),
            Request::DeleteRequestSchema(inner) => inner.into_command(),
            Request::DeleteResponseSchema(inner) => inner.into_command(),
            Request::Deliver(inner) => inner.into_command(),
            Request::DeliverRequestSchema(inner) => inner.into_command(),
            Request::DeliverResponseSchema(inner) => inner.into_command(),
            Request::Read(inner) => inner.into_command(),
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
        Request::Add(req) => {
            let value = add::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Add(value))))
        }
        Request::AddRequestSchema(req) => {
            let value = add::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::AddRequestSchema(value),
            )))
        }
        Request::AddResponseSchema(req) => {
            let value = add::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::AddResponseSchema(value),
            )))
        }
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
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    jq: String,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>,
    > = match request {
        Request::Add(req) => {
            let value = add::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::AddRequestSchema(req) => {
            let value =
                add::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::AddResponseSchema(req) => {
            let value =
                add::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Delete(req) => {
            let value = delete::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DeleteRequestSchema(req) => {
            let value =
                delete::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DeleteResponseSchema(req) => {
            let value =
                delete::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Deliver(req) => {
            let inner = deliver::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::DeliverRequestSchema(req) => {
            let value =
                deliver::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DeliverResponseSchema(req) => {
            let value =
                deliver::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Read(req) => {
            let inner = read::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(inner)
        }
    };
    Ok(stream)
}
