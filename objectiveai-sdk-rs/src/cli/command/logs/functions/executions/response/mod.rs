pub mod clear;
pub mod get;
pub mod list;
pub mod retry_tokens;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    Clear(clear::Command),
    Get(get::Command),
    List(list::Command),
    RetryTokens {
        #[command(subcommand)]
        command: retry_tokens::Command,
    },
    Subscribe(subscribe::Command),
}

#[derive(Debug, Clone)]
pub enum Request {
    Clear(clear::Request),
    ClearRequestSchema(clear::request_schema::Request),
    ClearResponseSchema(clear::response_schema::Request),
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    List(list::Request),
    ListRequestSchema(list::request_schema::Request),
    ListResponseSchema(list::response_schema::Request),
    RetryTokens(retry_tokens::Request),
    Subscribe(subscribe::Request),
    SubscribeRequestSchema(subscribe::request_schema::Request),
    SubscribeResponseSchema(subscribe::response_schema::Request),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ResponseItem {
    Clear(clear::Response),
    ClearRequestSchema(clear::request_schema::Response),
    ClearResponseSchema(clear::response_schema::Response),
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    List(list::ResponseItem),
    ListRequestSchema(list::request_schema::Response),
    ListResponseSchema(list::response_schema::Response),
    RetryTokens(retry_tokens::Response),
    Subscribe(subscribe::Response),
    SubscribeRequestSchema(subscribe::request_schema::Response),
    SubscribeResponseSchema(subscribe::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Clear(cmd) => match cmd.schema {
                None => Ok(Request::Clear(clear::Request::try_from(cmd.args)?)),
                Some(clear::Schema::RequestSchema(args)) =>
                    Ok(Request::ClearRequestSchema(clear::request_schema::Request::try_from(args)?)),
                Some(clear::Schema::ResponseSchema(args)) =>
                    Ok(Request::ClearResponseSchema(clear::response_schema::Request::try_from(args)?)),
            },
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) =>
                    Ok(Request::ListRequestSchema(list::request_schema::Request::try_from(args)?)),
                Some(list::Schema::ResponseSchema(args)) =>
                    Ok(Request::ListResponseSchema(list::response_schema::Request::try_from(args)?)),
            },
            Command::RetryTokens { command } =>
                Ok(Request::RetryTokens(retry_tokens::Request::try_from(command)?)),
            Command::Subscribe(cmd) => match cmd.schema {
                None => Ok(Request::Subscribe(subscribe::Request::try_from(cmd.args)?)),
                Some(subscribe::Schema::RequestSchema(args)) =>
                    Ok(Request::SubscribeRequestSchema(subscribe::request_schema::Request::try_from(args)?)),
                Some(subscribe::Schema::ResponseSchema(args)) =>
                    Ok(Request::SubscribeResponseSchema(subscribe::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Clear(inner) => inner.into_command(),
            Request::ClearRequestSchema(inner) => inner.into_command(),
            Request::ClearResponseSchema(inner) => inner.into_command(),
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
            Request::RetryTokens(inner) => inner.into_command(),
            Request::Subscribe(inner) => inner.into_command(),
            Request::SubscribeRequestSchema(inner) => inner.into_command(),
            Request::SubscribeResponseSchema(inner) => inner.into_command(),
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
            Request::Clear(req) => {
                let value = clear::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Clear(value),
                )))
            }
            Request::ClearRequestSchema(req) => {
                let value = clear::request_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ClearRequestSchema(value),
                )))
            }
            Request::ClearResponseSchema(req) => {
                let value = clear::response_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ClearResponseSchema(value),
                )))
            }
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
            Request::List(req) => {
                let inner = list::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::List)))
            }
            Request::ListRequestSchema(req) => {
                let value = list::request_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ListRequestSchema(value),
                )))
            }
            Request::ListResponseSchema(req) => {
                let value = list::response_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ListResponseSchema(value),
                )))
            }
            Request::RetryTokens(req) => {
                let inner = retry_tokens::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::RetryTokens)))
            }
            Request::Subscribe(req) => {
                let value = subscribe::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Subscribe(value),
                )))
            }
            Request::SubscribeRequestSchema(req) => {
                let value = subscribe::request_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SubscribeRequestSchema(value),
                )))
            }
            Request::SubscribeResponseSchema(req) => {
                let value = subscribe::response_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SubscribeResponseSchema(value),
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
            Request::Clear(req) => {
                let value = clear::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ClearRequestSchema(req) => {
                let value = clear::request_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ClearResponseSchema(req) => {
                let value = clear::response_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
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
            Request::List(req) => {
                let inner = list::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::ListRequestSchema(req) => {
                let value = list::request_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ListResponseSchema(req) => {
                let value = list::response_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::RetryTokens(req) => {
                let inner = retry_tokens::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Subscribe(req) => {
                let value = subscribe::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SubscribeRequestSchema(req) => {
                let value = subscribe::request_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SubscribeResponseSchema(req) => {
                let value = subscribe::response_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
