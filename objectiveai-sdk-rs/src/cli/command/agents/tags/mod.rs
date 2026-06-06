//! `agents tags` — client-side agent tags. Tags uniquely identify
//! one agent instance hierarchy and are stored locally (not on the
//! API server). Two leaves:
//!
//! - `lookup` — look up by `agent_instance_hierarchy` (returns the
//!   tag, if any) or by `tag` (returns the bound hierarchy, if any).
//! - `add` — register a tag in PENDING state for an
//!   `(agent_full_id, parent_agent_instance_hierarchy)` pair. The
//!   next agent-completion that matches the pair auto-binds the tag
//!   on its first chunk.

use crate::cli::command::CommandRequest;

pub mod add;
pub mod lookup;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Resolve a tag → agent-instance-hierarchy or vice versa.
    Lookup(lookup::Command),
    /// Register a tag in PENDING state (or refresh an existing tag).
    Add(add::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.tags.Request")]
pub enum Request {
    #[schemars(title = "Lookup")]
    Lookup(lookup::Request),
    #[schemars(title = "LookupRequestSchema")]
    LookupRequestSchema(lookup::request_schema::Request),
    #[schemars(title = "LookupResponseSchema")]
    LookupResponseSchema(lookup::response_schema::Request),
    #[schemars(title = "Add")]
    Add(add::Request),
    #[schemars(title = "AddRequestSchema")]
    AddRequestSchema(add::request_schema::Request),
    #[schemars(title = "AddResponseSchema")]
    AddResponseSchema(add::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Lookup")]
    Lookup(lookup::Response),
    #[schemars(title = "LookupRequestSchema")]
    LookupRequestSchema(lookup::request_schema::Response),
    #[schemars(title = "LookupResponseSchema")]
    LookupResponseSchema(lookup::response_schema::Response),
    #[schemars(title = "Add")]
    Add(add::Response),
    #[schemars(title = "AddRequestSchema")]
    AddRequestSchema(add::request_schema::Response),
    #[schemars(title = "AddResponseSchema")]
    AddResponseSchema(add::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Lookup(v) => v.into_mcp(),
            ResponseItem::LookupRequestSchema(v) => v.into_mcp(),
            ResponseItem::LookupResponseSchema(v) => v.into_mcp(),
            ResponseItem::Add(v) => v.into_mcp(),
            ResponseItem::AddRequestSchema(v) => v.into_mcp(),
            ResponseItem::AddResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Lookup(cmd) => match cmd.schema {
                None => Ok(Request::Lookup(lookup::Request::try_from(cmd.args)?)),
                Some(lookup::Schema::RequestSchema(args)) => Ok(
                    Request::LookupRequestSchema(lookup::request_schema::Request::try_from(args)?),
                ),
                Some(lookup::Schema::ResponseSchema(args)) => Ok(
                    Request::LookupResponseSchema(lookup::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Add(cmd) => match cmd.schema {
                None => Ok(Request::Add(add::Request::try_from(cmd.args)?)),
                Some(add::Schema::RequestSchema(args)) => Ok(
                    Request::AddRequestSchema(add::request_schema::Request::try_from(args)?),
                ),
                Some(add::Schema::ResponseSchema(args)) => Ok(
                    Request::AddResponseSchema(add::response_schema::Request::try_from(args)?),
                ),
            },
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Lookup(inner) => inner.into_command(),
            Request::LookupRequestSchema(inner) => inner.into_command(),
            Request::LookupResponseSchema(inner) => inner.into_command(),
            Request::Add(inner) => inner.into_command(),
            Request::AddRequestSchema(inner) => inner.into_command(),
            Request::AddResponseSchema(inner) => inner.into_command(),
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
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
        Request::Lookup(req) => {
            let value = lookup::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Lookup(value))))
        }
        Request::LookupRequestSchema(req) => {
            let value = lookup::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::LookupRequestSchema(value),
            )))
        }
        Request::LookupResponseSchema(req) => {
            let value = lookup::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::LookupResponseSchema(value),
            )))
        }
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
        Request::Lookup(req) => {
            let value = lookup::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::LookupRequestSchema(req) => {
            let value =
                lookup::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::LookupResponseSchema(req) => {
            let value =
                lookup::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
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
    };
    Ok(stream)
}
