pub mod get;
pub mod set;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Set(set::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    Set(set::Request),
    SetRequestSchema(set::request_schema::Request),
    SetResponseSchema(set::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    Set(set::Response),
    SetRequestSchema(set::request_schema::Response),
    SetResponseSchema(set::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
            Command::Set(cmd) => match cmd.schema {
                None => Ok(Request::Set(set::Request::try_from(cmd.args)?)),
                Some(set::Schema::RequestSchema(args)) =>
                    Ok(Request::SetRequestSchema(set::request_schema::Request::try_from(args)?)),
                Some(set::Schema::ResponseSchema(args)) =>
                    Ok(Request::SetResponseSchema(set::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Set(inner) => inner.into_command(),
            Request::SetRequestSchema(inner) => inner.into_command(),
            Request::SetResponseSchema(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>> =
        match request {
            Request::Get(req) => {
                let value = get::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Get(value),
                )))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::GetRequestSchema(value),
                )))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::GetResponseSchema(value),
                )))
            }
            Request::Set(req) => {
                let value = set::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Set(value),
                )))
            }
            Request::SetRequestSchema(req) => {
                let value = set::request_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SetRequestSchema(value),
                )))
            }
            Request::SetResponseSchema(req) => {
                let value = set::response_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SetResponseSchema(value),
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
            Request::Get(req) => {
                let value = get::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Set(req) => {
                let value = set::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SetRequestSchema(req) => {
                let value = set::request_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SetResponseSchema(req) => {
                let value = set::response_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
