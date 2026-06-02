pub mod agents;
pub mod functions;
pub mod mcp;
pub mod swarms;
pub mod viewer;

#[derive(clap::Subcommand)]
pub enum Command {
    Agents {
        #[command(subcommand)]
        command: agents::Command,
    },
    Functions {
        #[command(subcommand)]
        command: functions::Command,
    },
    Mcp {
        #[command(subcommand)]
        command: mcp::Command,
    },
    Swarms {
        #[command(subcommand)]
        command: swarms::Command,
    },
    Viewer {
        #[command(subcommand)]
        command: viewer::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Agents(agents::Request),
    Functions(functions::Request),
    Mcp(mcp::Request),
    Swarms(swarms::Request),
    Viewer(viewer::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Agents(agents::ResponseItem),
    Functions(functions::ResponseItem),
    Mcp(mcp::Response),
    Swarms(swarms::ResponseItem),
    Viewer(viewer::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Agents { command } =>
                Ok(Request::Agents(agents::Request::try_from(command)?)),
            Command::Functions { command } =>
                Ok(Request::Functions(functions::Request::try_from(command)?)),
            Command::Mcp { command } =>
                Ok(Request::Mcp(mcp::Request::try_from(command)?)),
            Command::Swarms { command } =>
                Ok(Request::Swarms(swarms::Request::try_from(command)?)),
            Command::Viewer { command } =>
                Ok(Request::Viewer(viewer::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Agents(inner) => inner.into_command(),
            Request::Functions(inner) => inner.into_command(),
            Request::Mcp(inner) => inner.into_command(),
            Request::Swarms(inner) => inner.into_command(),
            Request::Viewer(inner) => inner.into_command(),
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
            Request::Agents(req) => {
                let inner = agents::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Agents)))
            }
            Request::Functions(req) => {
                let inner = functions::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Functions)))
            }
            Request::Mcp(req) => {
                let inner = mcp::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
            }
            Request::Swarms(req) => {
                let inner = swarms::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Swarms)))
            }
            Request::Viewer(req) => {
                let inner = viewer::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Viewer)))
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
            Request::Agents(req) => {
                let inner = agents::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Functions(req) => {
                let inner = functions::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Mcp(req) => {
                let inner = mcp::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Swarms(req) => {
                let inner = swarms::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Viewer(req) => {
                let inner = viewer::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
