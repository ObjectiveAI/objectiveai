pub mod kill;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    Spawn(spawn::Command),
    Kill(kill::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.daemon.Request")]
pub enum Request {
    #[schemars(title = "Spawn")]
    Spawn(spawn::Request),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Request),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Request),
    #[schemars(title = "Kill")]
    Kill(kill::Request),
    #[schemars(title = "KillRequestSchema")]
    KillRequestSchema(kill::request_schema::Request),
    #[schemars(title = "KillResponseSchema")]
    KillResponseSchema(kill::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.daemon.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Spawn")]
    Spawn(spawn::ResponseItem),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Response),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Response),
    #[schemars(title = "Kill")]
    Kill(kill::Response),
    #[schemars(title = "KillRequestSchema")]
    KillRequestSchema(kill::request_schema::Response),
    #[schemars(title = "KillResponseSchema")]
    KillResponseSchema(kill::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Spawn(v) => v.into_mcp(),
            ResponseItem::SpawnRequestSchema(v) => v.into_mcp(),
            ResponseItem::SpawnResponseSchema(v) => v.into_mcp(),
            ResponseItem::Kill(v) => v.into_mcp(),
            ResponseItem::KillRequestSchema(v) => v.into_mcp(),
            ResponseItem::KillResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Spawn(cmd) => match cmd.schema {
                None => Ok(Request::Spawn(spawn::Request::try_from(cmd.args)?)),
                Some(spawn::Schema::RequestSchema(args)) =>
                    Ok(Request::SpawnRequestSchema(spawn::request_schema::Request::try_from(args)?)),
                Some(spawn::Schema::ResponseSchema(args)) =>
                    Ok(Request::SpawnResponseSchema(spawn::response_schema::Request::try_from(args)?)),
            },
            Command::Kill(cmd) => match cmd.schema {
                None => Ok(Request::Kill(kill::Request::try_from(cmd.args)?)),
                Some(kill::Schema::RequestSchema(args)) =>
                    Ok(Request::KillRequestSchema(kill::request_schema::Request::try_from(args)?)),
                Some(kill::Schema::ResponseSchema(args)) =>
                    Ok(Request::KillResponseSchema(kill::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Spawn(inner) => inner.request_base(),
            Request::SpawnRequestSchema(inner) => inner.request_base(),
            Request::SpawnResponseSchema(inner) => inner.request_base(),
            Request::Kill(inner) => inner.request_base(),
            Request::KillRequestSchema(inner) => inner.request_base(),
            Request::KillResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Spawn(inner) => inner.request_base_mut(),
            Request::SpawnRequestSchema(inner) => inner.request_base_mut(),
            Request::SpawnResponseSchema(inner) => inner.request_base_mut(),
            Request::Kill(inner) => inner.request_base_mut(),
            Request::KillRequestSchema(inner) => inner.request_base_mut(),
            Request::KillResponseSchema(inner) => inner.request_base_mut(),
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
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>> =
        match request {
            Request::Spawn(req) => {
                let inner = spawn::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Spawn)))
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
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Spawn(req) => {
                let inner = spawn::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::SpawnRequestSchema(req) => {
                let value = spawn::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SpawnResponseSchema(req) => {
                let value = spawn::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Kill(req) => {
                let value = kill::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::KillRequestSchema(req) => {
                let value = kill::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::KillResponseSchema(req) => {
                let value = kill::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Spawn(spawn::ListenerExecution),
    SpawnRequestSchema(spawn::request_schema::ListenerExecution),
    SpawnResponseSchema(spawn::response_schema::ListenerExecution),
    Kill(kill::ListenerExecution),
    KillRequestSchema(kill::request_schema::ListenerExecution),
    KillResponseSchema(kill::response_schema::ListenerExecution),
}
