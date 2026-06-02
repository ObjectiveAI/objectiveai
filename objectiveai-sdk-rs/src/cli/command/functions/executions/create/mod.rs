pub mod standard;
pub mod swiss_system;

#[derive(clap::Subcommand)]
pub enum Command {
    Standard(standard::Command),
    SwissSystem(swiss_system::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Standard(standard::Request),
    StandardRequestSchema(standard::request_schema::Request),
    StandardResponseSchema(standard::response_schema::Request),
    SwissSystem(swiss_system::Request),
    SwissSystemRequestSchema(swiss_system::request_schema::Request),
    SwissSystemResponseSchema(swiss_system::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Standard(standard::ResponseItem),
    StandardRequestSchema(standard::request_schema::Response),
    StandardResponseSchema(standard::response_schema::Response),
    SwissSystem(swiss_system::ResponseItem),
    SwissSystemRequestSchema(swiss_system::request_schema::Response),
    SwissSystemResponseSchema(swiss_system::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Standard(cmd) => match cmd.schema {
                None => Ok(Request::Standard(standard::Request::try_from(cmd.args)?)),
                Some(standard::Schema::RequestSchema(args)) =>
                    Ok(Request::StandardRequestSchema(standard::request_schema::Request::try_from(args)?)),
                Some(standard::Schema::ResponseSchema(args)) =>
                    Ok(Request::StandardResponseSchema(standard::response_schema::Request::try_from(args)?)),
            },
            Command::SwissSystem(cmd) => match cmd.schema {
                None => Ok(Request::SwissSystem(swiss_system::Request::try_from(cmd.args)?)),
                Some(swiss_system::Schema::RequestSchema(args)) =>
                    Ok(Request::SwissSystemRequestSchema(swiss_system::request_schema::Request::try_from(args)?)),
                Some(swiss_system::Schema::ResponseSchema(args)) =>
                    Ok(Request::SwissSystemResponseSchema(swiss_system::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Standard(inner) => inner.into_command(),
            Request::StandardRequestSchema(inner) => inner.into_command(),
            Request::StandardResponseSchema(inner) => inner.into_command(),
            Request::SwissSystem(inner) => inner.into_command(),
            Request::SwissSystemRequestSchema(inner) => inner.into_command(),
            Request::SwissSystemResponseSchema(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>> =
        match request {
            Request::Standard(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = standard::execute_streaming(executor, req).await?;
                    Box::pin(inner.map(|r| r.map(ResponseItem::Standard)))
                } else {
                    let value = standard::execute(executor, req).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(
                        ResponseItem::Standard(standard::ResponseItem::Id(value)),
                    )))
                }
            }
            Request::StandardRequestSchema(req) => {
                let value = standard::request_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::StandardRequestSchema(value),
                )))
            }
            Request::StandardResponseSchema(req) => {
                let value = standard::response_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::StandardResponseSchema(value),
                )))
            }
            Request::SwissSystem(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = swiss_system::execute_streaming(executor, req).await?;
                    Box::pin(inner.map(|r| r.map(ResponseItem::SwissSystem)))
                } else {
                    let value = swiss_system::execute(executor, req).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(
                        ResponseItem::SwissSystem(swiss_system::ResponseItem::Id(value)),
                    )))
                }
            }
            Request::SwissSystemRequestSchema(req) => {
                let value = swiss_system::request_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SwissSystemRequestSchema(value),
                )))
            }
            Request::SwissSystemResponseSchema(req) => {
                let value = swiss_system::response_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SwissSystemResponseSchema(value),
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
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Standard(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = standard::execute_streaming_jq(executor, req, jq).await?;
                    Box::pin(inner)
                } else {
                    let value = standard::execute_jq(executor, req, jq).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
                }
            }
            Request::StandardRequestSchema(req) => {
                let value = standard::request_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::StandardResponseSchema(req) => {
                let value = standard::response_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SwissSystem(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = swiss_system::execute_streaming_jq(executor, req, jq).await?;
                    Box::pin(inner)
                } else {
                    let value = swiss_system::execute_jq(executor, req, jq).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
                }
            }
            Request::SwissSystemRequestSchema(req) => {
                let value = swiss_system::request_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SwissSystemResponseSchema(req) => {
                let value = swiss_system::response_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
