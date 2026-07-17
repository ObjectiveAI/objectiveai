pub mod request;

#[derive(clap::Subcommand)]
pub enum Command {
    Request(request::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.user.Request")]
pub enum Request {
    #[schemars(title = "Request")]
    Request(request::Request),
    #[schemars(title = "RequestRequestSchema")]
    RequestRequestSchema(request::request_schema::Request),
    #[schemars(title = "RequestResponseSchema")]
    RequestResponseSchema(request::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.user.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Request")]
    Request(request::Response),
    #[schemars(title = "RequestRequestSchema")]
    RequestRequestSchema(request::request_schema::Response),
    #[schemars(title = "RequestResponseSchema")]
    RequestResponseSchema(request::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Request(v) => v.into_mcp(),
            Response::RequestRequestSchema(v) => v.into_mcp(),
            Response::RequestResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Request(cmd) => match cmd.schema {
                None => Ok(Request::Request(request::Request::try_from(cmd.args)?)),
                Some(request::Schema::RequestSchema(args)) =>
                    Ok(Request::RequestRequestSchema(request::request_schema::Request::try_from(args)?)),
                Some(request::Schema::ResponseSchema(args)) =>
                    Ok(Request::RequestResponseSchema(request::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Request(inner) => inner.request_base(),
            Request::RequestRequestSchema(inner) => inner.request_base(),
            Request::RequestResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Request(inner) => inner.request_base_mut(),
            Request::RequestRequestSchema(inner) => inner.request_base_mut(),
            Request::RequestResponseSchema(inner) => inner.request_base_mut(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>> =
        match request {
            Request::Request(req) => {
                let value = request::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Request(value),
                )))
            }
            Request::RequestRequestSchema(req) => {
                let value = request::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::RequestRequestSchema(value),
                )))
            }
            Request::RequestResponseSchema(req) => {
                let value = request::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::RequestResponseSchema(value),
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
            Request::Request(req) => {
                let value = request::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::RequestRequestSchema(req) => {
                let value = request::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::RequestResponseSchema(req) => {
                let value = request::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Request(request::ListenerExecution),
    RequestRequestSchema(request::request_schema::ListenerExecution),
    RequestResponseSchema(request::response_schema::ListenerExecution),
}
