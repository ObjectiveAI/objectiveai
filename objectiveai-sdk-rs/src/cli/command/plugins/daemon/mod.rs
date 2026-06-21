pub mod notify;

#[derive(clap::Subcommand)]
pub enum Command {
    Notify(notify::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.plugins.daemon.Request")]
pub enum Request {
    #[schemars(title = "Notify")]
    Notify(notify::Request),
    #[schemars(title = "NotifyRequestSchema")]
    NotifyRequestSchema(notify::request_schema::Request),
    #[schemars(title = "NotifyResponseSchema")]
    NotifyResponseSchema(notify::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.daemon.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Notify")]
    Notify(notify::Response),
    #[schemars(title = "NotifyRequestSchema")]
    NotifyRequestSchema(notify::request_schema::Response),
    #[schemars(title = "NotifyResponseSchema")]
    NotifyResponseSchema(notify::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Notify(v) => v.into_mcp(),
            Response::NotifyRequestSchema(v) => v.into_mcp(),
            Response::NotifyResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Notify(cmd) => match cmd.schema {
                None => Ok(Request::Notify(notify::Request::try_from(cmd.args)?)),
                Some(notify::Schema::RequestSchema(args)) =>
                    Ok(Request::NotifyRequestSchema(notify::request_schema::Request::try_from(args)?)),
                Some(notify::Schema::ResponseSchema(args)) =>
                    Ok(Request::NotifyResponseSchema(notify::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Notify(inner) => inner.request_base(),
            Request::NotifyRequestSchema(inner) => inner.request_base(),
            Request::NotifyResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Notify(inner) => inner.request_base_mut(),
            Request::NotifyRequestSchema(inner) => inner.request_base_mut(),
            Request::NotifyResponseSchema(inner) => inner.request_base_mut(),
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
            Request::Notify(req) => {
                let value = notify::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(Response::Notify(value))))
            }
            Request::NotifyRequestSchema(req) => {
                let value = notify::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::NotifyRequestSchema(value),
                )))
            }
            Request::NotifyResponseSchema(req) => {
                let value = notify::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::NotifyResponseSchema(value),
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
            Request::Notify(req) => {
                let value = notify::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::NotifyRequestSchema(req) => {
                let value = notify::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::NotifyResponseSchema(req) => {
                let value = notify::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
