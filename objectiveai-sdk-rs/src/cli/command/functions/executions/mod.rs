pub mod create;

#[derive(clap::Subcommand)]
pub enum Command {
    Create {
        #[command(subcommand)]
        command: create::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Create(create::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Create(create::ResponseItem),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Create { command } =>
                Ok(Request::Create(create::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Create(inner) => inner.into_command(),
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
            Request::Create(req) => {
                let inner = create::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Create)))
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
            Request::Create(req) => {
                let inner = create::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
