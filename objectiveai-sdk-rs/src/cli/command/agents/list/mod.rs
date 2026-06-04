pub mod active;
pub mod available;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Direct children of the calling agent.
    Active(active::Command),
    /// Remote agents available from a given source.
    Available(available::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Active(active::Request),
    ActiveRequestSchema(active::request_schema::Request),
    ActiveResponseSchema(active::response_schema::Request),
    Available(available::Request),
    AvailableRequestSchema(available::request_schema::Request),
    AvailableResponseSchema(available::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ResponseItem {
    Active(active::ResponseItem),
    ActiveRequestSchema(active::request_schema::Response),
    ActiveResponseSchema(active::response_schema::Response),
    Available(available::ResponseItem),
    AvailableRequestSchema(available::request_schema::Response),
    AvailableResponseSchema(available::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Active(v) => v.into_mcp(),
            ResponseItem::ActiveRequestSchema(v) => v.into_mcp(),
            ResponseItem::ActiveResponseSchema(v) => v.into_mcp(),
            ResponseItem::Available(v) => v.into_mcp(),
            ResponseItem::AvailableRequestSchema(v) => v.into_mcp(),
            ResponseItem::AvailableResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Active(cmd) => match cmd.schema {
                None => Ok(Request::Active(active::Request::try_from(cmd.args)?)),
                Some(active::Schema::RequestSchema(args)) =>
                    Ok(Request::ActiveRequestSchema(active::request_schema::Request::try_from(args)?)),
                Some(active::Schema::ResponseSchema(args)) =>
                    Ok(Request::ActiveResponseSchema(active::response_schema::Request::try_from(args)?)),
            },
            Command::Available(cmd) => match cmd.schema {
                None => Ok(Request::Available(available::Request::try_from(cmd.args)?)),
                Some(available::Schema::RequestSchema(args)) =>
                    Ok(Request::AvailableRequestSchema(available::request_schema::Request::try_from(args)?)),
                Some(available::Schema::ResponseSchema(args)) =>
                    Ok(Request::AvailableResponseSchema(available::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Active(inner) => inner.into_command(),
            Request::ActiveRequestSchema(inner) => inner.into_command(),
            Request::ActiveResponseSchema(inner) => inner.into_command(),
            Request::Available(inner) => inner.into_command(),
            Request::AvailableRequestSchema(inner) => inner.into_command(),
            Request::AvailableResponseSchema(inner) => inner.into_command(),
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
            Request::Active(req) => {
                let inner = active::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Active)))
            }
            Request::ActiveRequestSchema(req) => {
                let value = active::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ActiveRequestSchema(value),
                )))
            }
            Request::ActiveResponseSchema(req) => {
                let value = active::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ActiveResponseSchema(value),
                )))
            }
            Request::Available(req) => {
                let inner = available::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Available)))
            }
            Request::AvailableRequestSchema(req) => {
                let value = available::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::AvailableRequestSchema(value),
                )))
            }
            Request::AvailableResponseSchema(req) => {
                let value = available::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::AvailableResponseSchema(value),
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
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Active(req) => {
                let inner = active::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::ActiveRequestSchema(req) => {
                let value = active::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ActiveResponseSchema(req) => {
                let value = active::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Available(req) => {
                let inner = available::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::AvailableRequestSchema(req) => {
                let value = available::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::AvailableResponseSchema(req) => {
                let value = available::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
