pub mod add;
pub mod del;
pub mod get;

#[derive(clap::Subcommand)]
pub enum Command {
    Add(add::Command),
    Del(del::Command),
    Get(get::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.laboratories.config.addresses.Request")]
pub enum Request {
    #[schemars(title = "Add")]
    Add(add::Request),
    #[schemars(title = "AddRequestSchema")]
    AddRequestSchema(add::request_schema::Request),
    #[schemars(title = "AddResponseSchema")]
    AddResponseSchema(add::response_schema::Request),
    #[schemars(title = "Del")]
    Del(del::Request),
    #[schemars(title = "DelRequestSchema")]
    DelRequestSchema(del::request_schema::Request),
    #[schemars(title = "DelResponseSchema")]
    DelResponseSchema(del::response_schema::Request),
    #[schemars(title = "Get")]
    Get(get::Request),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Request),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.config.addresses.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Add")]
    Add(add::Response),
    #[schemars(title = "AddRequestSchema")]
    AddRequestSchema(add::request_schema::Response),
    #[schemars(title = "AddResponseSchema")]
    AddResponseSchema(add::response_schema::Response),
    #[schemars(title = "Del")]
    Del(del::Response),
    #[schemars(title = "DelRequestSchema")]
    DelRequestSchema(del::request_schema::Response),
    #[schemars(title = "DelResponseSchema")]
    DelResponseSchema(del::response_schema::Response),
    #[schemars(title = "Get")]
    Get(get::Response),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Response),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Add(v) => v.into_mcp(),
            Response::AddRequestSchema(v) => v.into_mcp(),
            Response::AddResponseSchema(v) => v.into_mcp(),
            Response::Del(v) => v.into_mcp(),
            Response::DelRequestSchema(v) => v.into_mcp(),
            Response::DelResponseSchema(v) => v.into_mcp(),
            Response::Get(v) => v.into_mcp(),
            Response::GetRequestSchema(v) => v.into_mcp(),
            Response::GetResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Add(cmd) => match cmd.schema {
                None => Ok(Request::Add(add::Request::try_from(cmd.args)?)),
                Some(add::Schema::RequestSchema(args)) =>
                    Ok(Request::AddRequestSchema(add::request_schema::Request::try_from(args)?)),
                Some(add::Schema::ResponseSchema(args)) =>
                    Ok(Request::AddResponseSchema(add::response_schema::Request::try_from(args)?)),
            },
            Command::Del(cmd) => match cmd.schema {
                None => Ok(Request::Del(del::Request::try_from(cmd.args)?)),
                Some(del::Schema::RequestSchema(args)) =>
                    Ok(Request::DelRequestSchema(del::request_schema::Request::try_from(args)?)),
                Some(del::Schema::ResponseSchema(args)) =>
                    Ok(Request::DelResponseSchema(del::response_schema::Request::try_from(args)?)),
            },
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Add(inner) => inner.request_base(),
            Request::AddRequestSchema(inner) => inner.request_base(),
            Request::AddResponseSchema(inner) => inner.request_base(),
            Request::Del(inner) => inner.request_base(),
            Request::DelRequestSchema(inner) => inner.request_base(),
            Request::DelResponseSchema(inner) => inner.request_base(),
            Request::Get(inner) => inner.request_base(),
            Request::GetRequestSchema(inner) => inner.request_base(),
            Request::GetResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Add(inner) => inner.request_base_mut(),
            Request::AddRequestSchema(inner) => inner.request_base_mut(),
            Request::AddResponseSchema(inner) => inner.request_base_mut(),
            Request::Del(inner) => inner.request_base_mut(),
            Request::DelRequestSchema(inner) => inner.request_base_mut(),
            Request::DelResponseSchema(inner) => inner.request_base_mut(),
            Request::Get(inner) => inner.request_base_mut(),
            Request::GetRequestSchema(inner) => inner.request_base_mut(),
            Request::GetResponseSchema(inner) => inner.request_base_mut(),
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
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>> =
        match request {
            Request::Add(req) => {
                let value = add::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Add(value),
                )))
            }
            Request::AddRequestSchema(req) => {
                let value = add::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::AddRequestSchema(value),
                )))
            }
            Request::AddResponseSchema(req) => {
                let value = add::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::AddResponseSchema(value),
                )))
            }
            Request::Del(req) => {
                let value = del::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Del(value),
                )))
            }
            Request::DelRequestSchema(req) => {
                let value = del::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::DelRequestSchema(value),
                )))
            }
            Request::DelResponseSchema(req) => {
                let value = del::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::DelResponseSchema(value),
                )))
            }
            Request::Get(req) => {
                let value = get::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Get(value),
                )))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GetRequestSchema(value),
                )))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GetResponseSchema(value),
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
            Request::Add(req) => {
                let value = add::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::AddRequestSchema(req) => {
                let value = add::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::AddResponseSchema(req) => {
                let value = add::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Del(req) => {
                let value = del::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::DelRequestSchema(req) => {
                let value = del::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::DelResponseSchema(req) => {
                let value = del::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Get(req) => {
                let value = get::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Add(add::ListenerExecution),
    AddRequestSchema(add::request_schema::ListenerExecution),
    AddResponseSchema(add::response_schema::ListenerExecution),
    Del(del::ListenerExecution),
    DelRequestSchema(del::request_schema::ListenerExecution),
    DelResponseSchema(del::response_schema::ListenerExecution),
    Get(get::ListenerExecution),
    GetRequestSchema(get::request_schema::ListenerExecution),
    GetResponseSchema(get::response_schema::ListenerExecution),
}
