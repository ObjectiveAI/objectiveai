pub mod recursive;
pub mod state;

#[derive(clap::Subcommand)]
pub enum Command {
    Recursive {
        #[command(subcommand)]
        command: recursive::Command,
    },
    State {
        #[command(subcommand)]
        command: state::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.functions.inventions.Request")]
pub enum Request {
    #[schemars(title = "Recursive")]
    Recursive(recursive::Request),
    #[schemars(title = "State")]
    State(state::Request),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.inventions.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Recursive")]
    Recursive(recursive::ResponseItem),
    #[schemars(title = "State")]
    State(state::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Recursive(v) => v.into_mcp(),
            ResponseItem::State(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Recursive { command } =>
                Ok(Request::Recursive(recursive::Request::try_from(command)?)),
            Command::State { command } =>
                Ok(Request::State(state::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Recursive(inner) => inner.into_command(),
            Request::State(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>> =
        match request {
            Request::Recursive(req) => {
                let inner = recursive::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Recursive)))
            }
            Request::State(req) => {
                let inner = state::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::State)))
            }
        };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Recursive(req) => {
                let inner = recursive::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::State(req) => {
                let inner = state::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
