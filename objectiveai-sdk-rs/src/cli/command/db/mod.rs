//! `db` — direct database access at the CLI level.
//!
//! One leaf today:
//! - `query` — execute arbitrary single-statement read-only SQL
//!   with a required timeout and an optional per-response token
//!   budget.

use crate::cli::command::CommandRequest;

pub mod query;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Execute an arbitrary single-statement read-only SQL query
    /// against the CLI's local postgres pool.
    Query(query::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.db.Request")]
pub enum Request {
    #[schemars(title = "Query")]
    Query(query::Request),
    #[schemars(title = "QueryRequestSchema")]
    QueryRequestSchema(query::request_schema::Request),
    #[schemars(title = "QueryResponseSchema")]
    QueryResponseSchema(query::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.db.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Query")]
    Query(query::Response),
    #[schemars(title = "QueryRequestSchema")]
    QueryRequestSchema(query::request_schema::Response),
    #[schemars(title = "QueryResponseSchema")]
    QueryResponseSchema(query::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Query(v) => v.into_mcp(),
            ResponseItem::QueryRequestSchema(v) => v.into_mcp(),
            ResponseItem::QueryResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Query(cmd) => match cmd.schema {
                None => Ok(Request::Query(query::Request::try_from(cmd.args)?)),
                Some(query::Schema::RequestSchema(args)) => Ok(
                    Request::QueryRequestSchema(query::request_schema::Request::try_from(args)?),
                ),
                Some(query::Schema::ResponseSchema(args)) => Ok(
                    Request::QueryResponseSchema(query::response_schema::Request::try_from(args)?),
                ),
            },
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Query(inner) => inner.into_command(),
            Request::QueryRequestSchema(inner) => inner.into_command(),
            Request::QueryResponseSchema(inner) => inner.into_command(),
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
        Request::Query(req) => {
            let value = query::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Query(value),
            )))
        }
        Request::QueryRequestSchema(req) => {
            let value = query::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::QueryRequestSchema(value),
            )))
        }
        Request::QueryResponseSchema(req) => {
            let value = query::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::QueryResponseSchema(value),
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
        Request::Query(req) => {
            let value = query::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::QueryRequestSchema(req) => {
            let value =
                query::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::QueryResponseSchema(req) => {
            let value =
                query::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}
