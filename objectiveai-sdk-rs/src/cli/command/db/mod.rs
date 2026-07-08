//! `db` — direct database access at the CLI level.
//!
//! Leaves:
//! - `query` — execute arbitrary single-statement read-only SQL
//!   with a required timeout and an optional per-response token
//!   budget.
//! - `spawn` — start the `objectiveai-db` postgres vehicle using
//!   `config db` connection settings.
//! - `kill` — stop the postmaster started by `db spawn`.

use crate::cli::command::CommandRequest;

pub mod config;
pub mod kill;
pub mod query;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    Config {
        #[command(subcommand)]
        command: config::Command,
    },
    /// Stop the postmaster started by `db spawn`.
    /// Idempotent — succeeds with count = 0 if none was running.
    Kill(kill::Command),
    /// Execute an arbitrary single-statement read-only SQL query
    /// against the configured postgres pool.
    Query(query::Command),
    /// Start the `objectiveai-db` server in the background using
    /// `config db` connection settings. Errors if something is
    /// already listening there.
    Spawn(spawn::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.db.Request")]
pub enum Request {
    #[schemars(title = "Config")]
    Config(config::Request),
    #[schemars(title = "Kill")]
    Kill(kill::Request),
    #[schemars(title = "KillRequestSchema")]
    KillRequestSchema(kill::request_schema::Request),
    #[schemars(title = "KillResponseSchema")]
    KillResponseSchema(kill::response_schema::Request),
    #[schemars(title = "Query")]
    Query(query::Request),
    #[schemars(title = "QueryRequestSchema")]
    QueryRequestSchema(query::request_schema::Request),
    #[schemars(title = "QueryResponseSchema")]
    QueryResponseSchema(query::response_schema::Request),
    #[schemars(title = "Spawn")]
    Spawn(spawn::Request),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Request),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.db.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Config")]
    Config(config::Response),
    #[schemars(title = "Kill")]
    Kill(kill::Response),
    #[schemars(title = "KillRequestSchema")]
    KillRequestSchema(kill::request_schema::Response),
    #[schemars(title = "KillResponseSchema")]
    KillResponseSchema(kill::response_schema::Response),
    #[schemars(title = "Query")]
    Query(query::Response),
    #[schemars(title = "QueryRequestSchema")]
    QueryRequestSchema(query::request_schema::Response),
    #[schemars(title = "QueryResponseSchema")]
    QueryResponseSchema(query::response_schema::Response),
    #[schemars(title = "Spawn")]
    Spawn(spawn::Response),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Response),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Config(v) => v.into_mcp(),
            ResponseItem::Kill(v) => v.into_mcp(),
            ResponseItem::KillRequestSchema(v) => v.into_mcp(),
            ResponseItem::KillResponseSchema(v) => v.into_mcp(),
            ResponseItem::Query(v) => v.into_mcp(),
            ResponseItem::QueryRequestSchema(v) => v.into_mcp(),
            ResponseItem::QueryResponseSchema(v) => v.into_mcp(),
            ResponseItem::Spawn(v) => v.into_mcp(),
            ResponseItem::SpawnRequestSchema(v) => v.into_mcp(),
            ResponseItem::SpawnResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Config { command } =>
                Ok(Request::Config(config::Request::try_from(command)?)),
            Command::Kill(cmd) => match cmd.schema {
                None => Ok(Request::Kill(kill::Request::try_from(cmd.args)?)),
                Some(kill::Schema::RequestSchema(args)) => Ok(
                    Request::KillRequestSchema(kill::request_schema::Request::try_from(args)?),
                ),
                Some(kill::Schema::ResponseSchema(args)) => Ok(
                    Request::KillResponseSchema(kill::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Query(cmd) => match cmd.schema {
                None => Ok(Request::Query(query::Request::try_from(cmd.args)?)),
                Some(query::Schema::RequestSchema(args)) => Ok(
                    Request::QueryRequestSchema(query::request_schema::Request::try_from(args)?),
                ),
                Some(query::Schema::ResponseSchema(args)) => Ok(
                    Request::QueryResponseSchema(query::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Spawn(cmd) => match cmd.schema {
                None => Ok(Request::Spawn(spawn::Request::try_from(cmd.args)?)),
                Some(spawn::Schema::RequestSchema(args)) => Ok(
                    Request::SpawnRequestSchema(spawn::request_schema::Request::try_from(args)?),
                ),
                Some(spawn::Schema::ResponseSchema(args)) => Ok(
                    Request::SpawnResponseSchema(spawn::response_schema::Request::try_from(args)?),
                ),
            },
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Config(inner) => inner.request_base(),
            Request::Kill(inner) => inner.request_base(),
            Request::KillRequestSchema(inner) => inner.request_base(),
            Request::KillResponseSchema(inner) => inner.request_base(),
            Request::Query(inner) => inner.request_base(),
            Request::QueryRequestSchema(inner) => inner.request_base(),
            Request::QueryResponseSchema(inner) => inner.request_base(),
            Request::Spawn(inner) => inner.request_base(),
            Request::SpawnRequestSchema(inner) => inner.request_base(),
            Request::SpawnResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Config(inner) => inner.request_base_mut(),
            Request::Kill(inner) => inner.request_base_mut(),
            Request::KillRequestSchema(inner) => inner.request_base_mut(),
            Request::KillResponseSchema(inner) => inner.request_base_mut(),
            Request::Query(inner) => inner.request_base_mut(),
            Request::QueryRequestSchema(inner) => inner.request_base_mut(),
            Request::QueryResponseSchema(inner) => inner.request_base_mut(),
            Request::Spawn(inner) => inner.request_base_mut(),
            Request::SpawnRequestSchema(inner) => inner.request_base_mut(),
            Request::SpawnResponseSchema(inner) => inner.request_base_mut(),
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
            Request::Config(req) => {
                let inner = config::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Config)))
            }
        Request::Kill(req) => {
            let value = kill::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Kill(value),
            )))
        }
        Request::KillRequestSchema(req) => {
            let value = kill::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::KillRequestSchema(value),
            )))
        }
        Request::KillResponseSchema(req) => {
            let value = kill::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::KillResponseSchema(value),
            )))
        }
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
        Request::Spawn(req) => {
            let value = spawn::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Spawn(value),
            )))
        }
        Request::SpawnRequestSchema(req) => {
            let value = spawn::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::SpawnRequestSchema(value),
            )))
        }
        Request::SpawnResponseSchema(req) => {
            let value = spawn::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::SpawnResponseSchema(value),
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
            Request::Config(req) => {
                let inner = config::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
        Request::Kill(req) => {
            let value = kill::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::KillRequestSchema(req) => {
            let value =
                kill::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::KillResponseSchema(req) => {
            let value =
                kill::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Query(req) => {
            let value = query::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::QueryRequestSchema(req) => {
            let value =
                query::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::QueryResponseSchema(req) => {
            let value =
                query::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Spawn(req) => {
            let value = spawn::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::SpawnRequestSchema(req) => {
            let value =
                spawn::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::SpawnResponseSchema(req) => {
            let value =
                spawn::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Config(config::ListenerExecution),
    Kill(kill::ListenerExecution),
    KillRequestSchema(kill::request_schema::ListenerExecution),
    KillResponseSchema(kill::response_schema::ListenerExecution),
    Query(query::ListenerExecution),
    QueryRequestSchema(query::request_schema::ListenerExecution),
    QueryResponseSchema(query::response_schema::ListenerExecution),
    Spawn(spawn::ListenerExecution),
    SpawnRequestSchema(spawn::request_schema::ListenerExecution),
    SpawnResponseSchema(spawn::response_schema::ListenerExecution),
}
