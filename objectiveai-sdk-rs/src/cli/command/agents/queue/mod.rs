//! `agents queue` — deferred prompts queue. Two top-level subcommands:
//!
//! - `add` — write-only enqueue. Stores a prompt in `tags.sqlite`
//!   (the `prompts` table) against either a resolved
//!   `agent_instance_hierarchy` (Direct mode) or a literal
//!   `agent_tag` (Tag mode — no resolution at enqueue time).
//! - `list` — streaming inspection. Both Direct and Tag rows are
//!   filtered to direct children of a parent (Tag rows resolve
//!   their parent via the joined `tags` table). Each tag-row item
//!   carries the joined 3-state status and the resolved prompt
//!   body as `Vec<ResponseQueueMessage>` (id-referenced into the
//!   per-kind content tables).
//! - `read` (nested) — sub-tier whose only leaf today is `id`,
//!   which fetches one piece of queued content by its
//!   `prompt_contents.id`. The wire shape mirrors `RichContentPart`
//!   (tagged by `type`).
//!
//! Dequeue / delete leaves will land in follow-up passes.

use crate::cli::command::CommandRequest;

pub mod add;
pub mod list;
pub mod read;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Enqueue a prompt against a target agent.
    Add(add::Command),
    /// List queued prompts visible under a parent.
    List(list::Command),
    /// Read queued content by id (`agents queue read id <id>`).
    Read(ReadCommand),
}

/// Intermediate clap level for the `read` sub-tier — its only
/// subcommand today is `id`. Splitting it into its own wrapper
/// (rather than a fattened `ReadId` variant on [`Command`]) gives
/// the CLI surface `agents queue read id <num>` to match the user's
/// invocation style and keeps the door open for additional
/// `read <…>` leaves later.
#[derive(clap::Args)]
pub struct ReadCommand {
    #[command(subcommand)]
    pub sub: read::Command,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.queue.Request")]
pub enum Request {
    #[schemars(title = "Add")]
    Add(add::Request),
    #[schemars(title = "AddRequestSchema")]
    AddRequestSchema(add::request_schema::Request),
    #[schemars(title = "AddResponseSchema")]
    AddResponseSchema(add::response_schema::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
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
    #[schemars(title = "Add")]
    Add(add::Response),
    #[schemars(title = "AddRequestSchema")]
    AddRequestSchema(add::request_schema::Response),
    #[schemars(title = "AddResponseSchema")]
    AddResponseSchema(add::response_schema::Response),
    #[schemars(title = "List")]
    List(list::ResponseItem),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
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
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
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
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) => Ok(
                    Request::ListRequestSchema(list::request_schema::Request::try_from(args)?),
                ),
                Some(list::Schema::ResponseSchema(args)) => Ok(
                    Request::ListResponseSchema(list::response_schema::Request::try_from(args)?),
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
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
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
        Request::List(req) => {
            let inner = list::execute(executor, req, agent_arguments).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListRequestSchema(value),
            )))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListResponseSchema(value),
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
        Request::List(req) => {
            let inner = list::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::ListRequestSchema(req) => {
            let value =
                list::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListResponseSchema(req) => {
            let value =
                list::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Read(req) => {
            let inner = read::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(inner)
        }
    };
    Ok(stream)
}
