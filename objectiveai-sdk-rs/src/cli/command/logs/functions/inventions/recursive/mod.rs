pub mod request;
pub mod response;

#[derive(clap::Subcommand)]
pub enum Command {
    Request {
        #[command(subcommand)]
        command: request::Command,
    },
    Response {
        #[command(subcommand)]
        command: response::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Request(request::Request),
    Response(response::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Request(request::Response),
    Response(response::ResponseItem),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Request { command } =>
                Ok(Request::Request(request::Request::try_from(command)?)),
            Command::Response { command } =>
                Ok(Request::Response(response::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Request(inner) => inner.into_command(),
            Request::Response(inner) => inner.into_command(),
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
            Request::Request(req) => {
                let inner = request::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Request)))
            }
            Request::Response(req) => {
                let inner = response::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Response)))
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
            Request::Request(req) => {
                let inner = request::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Response(req) => {
                let inner = response::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
