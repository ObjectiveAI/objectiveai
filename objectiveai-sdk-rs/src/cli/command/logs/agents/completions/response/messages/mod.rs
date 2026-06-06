pub mod assistant;
pub mod tool;

#[derive(clap::Subcommand)]
pub enum Command {
    Assistant {
        #[command(subcommand)]
        command: assistant::Command,
    },
    Tool {
        #[command(subcommand)]
        command: tool::Command,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.logs.agents.completions.response.messages.Request")]
pub enum Request {
    #[schemars(title = "Assistant")]
    Assistant(assistant::Request),
    #[schemars(title = "Tool")]
    Tool(tool::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.logs.agents.completions.response.messages.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Assistant")]
    Assistant(assistant::Response),
    #[schemars(title = "Tool")]
    Tool(tool::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Assistant(v) => v.into_mcp(),
            Response::Tool(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Assistant { command } =>
                Ok(Request::Assistant(assistant::Request::try_from(command)?)),
            Command::Tool { command } =>
                Ok(Request::Tool(tool::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Assistant(inner) => inner.into_command(),
            Request::Tool(inner) => inner.into_command(),
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
            Request::Assistant(req) => {
                let inner = assistant::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Assistant)))
            }
            Request::Tool(req) => {
                let inner = tool::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Tool)))
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
            Request::Assistant(req) => {
                let inner = assistant::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Tool(req) => {
                let inner = tool::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
