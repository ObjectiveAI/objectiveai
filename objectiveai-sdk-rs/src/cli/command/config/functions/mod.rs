pub mod favorites;
pub mod get;
pub mod inventions;
pub mod profiles;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Favorites {
        #[command(subcommand)]
        command: favorites::Command,
    },
    Inventions {
        #[command(subcommand)]
        command: inventions::Command,
    },
    Profiles {
        #[command(subcommand)]
        command: profiles::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    Favorites(favorites::Request),
    Inventions(inventions::Request),
    Profiles(profiles::Request),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ResponseItem {
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    Favorites(favorites::ResponseItem),
    Inventions(inventions::Response),
    Profiles(profiles::ResponseItem),
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
            Command::Favorites { command } =>
                Ok(Request::Favorites(favorites::Request::try_from(command)?)),
            Command::Inventions { command } =>
                Ok(Request::Inventions(inventions::Request::try_from(command)?)),
            Command::Profiles { command } =>
                Ok(Request::Profiles(profiles::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Favorites(inner) => inner.into_command(),
            Request::Inventions(inner) => inner.into_command(),
            Request::Profiles(inner) => inner.into_command(),
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
            Request::Favorites(req) => {
                let inner = favorites::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Favorites)))
            }
            Request::Inventions(req) => {
                let inner = inventions::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Inventions)))
            }
            Request::Profiles(req) => {
                let inner = profiles::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Profiles)))
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
            Request::Favorites(req) => {
                let inner = favorites::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Inventions(req) => {
                let inner = inventions::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Profiles(req) => {
                let inner = profiles::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
