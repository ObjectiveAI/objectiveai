pub mod executions;
pub mod inventions;

#[derive(clap::Subcommand)]
pub enum Command {
    Executions {
        #[command(subcommand)]
        command: executions::Command,
    },
    Inventions {
        #[command(subcommand)]
        command: inventions::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Executions(executions::Request),
    Inventions(inventions::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Executions(executions::ResponseItem),
    Inventions(inventions::ResponseItem),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Executions { command } =>
                Ok(Request::Executions(executions::Request::try_from(command)?)),
            Command::Inventions { command } =>
                Ok(Request::Inventions(inventions::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Executions(inner) => inner.into_command(),
            Request::Inventions(inner) => inner.into_command(),
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
            Request::Executions(req) => {
                let inner = executions::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Executions)))
            }
            Request::Inventions(req) => {
                let inner = inventions::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Inventions)))
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
            Request::Executions(req) => {
                let inner = executions::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Inventions(req) => {
                let inner = inventions::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
