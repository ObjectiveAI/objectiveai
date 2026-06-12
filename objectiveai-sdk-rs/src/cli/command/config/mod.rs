mod scope;
pub use scope::*;

pub mod api;
pub mod db;
pub mod mcp;
pub mod viewer;

#[derive(clap::Subcommand)]
pub enum Command {
    Api {
        #[command(subcommand)]
        command: api::Command,
    },
    Db {
        #[command(subcommand)]
        command: db::Command,
    },
    Mcp {
        #[command(subcommand)]
        command: mcp::Command,
    },
    Viewer {
        #[command(subcommand)]
        command: viewer::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.config.Request")]
pub enum Request {
    #[schemars(title = "Api")]
    Api(api::Request),
    #[schemars(title = "Db")]
    Db(db::Request),
    #[schemars(title = "Mcp")]
    Mcp(mcp::Request),
    #[schemars(title = "Viewer")]
    Viewer(viewer::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Api")]
    Api(api::Response),
    #[schemars(title = "Db")]
    Db(db::Response),
    #[schemars(title = "Mcp")]
    Mcp(mcp::Response),
    #[schemars(title = "Viewer")]
    Viewer(viewer::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Api(v) => v.into_mcp(),
            ResponseItem::Db(v) => v.into_mcp(),
            ResponseItem::Mcp(v) => v.into_mcp(),
            ResponseItem::Viewer(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Api { command } =>
                Ok(Request::Api(api::Request::try_from(command)?)),
            Command::Db { command } =>
                Ok(Request::Db(db::Request::try_from(command)?)),
            Command::Mcp { command } =>
                Ok(Request::Mcp(mcp::Request::try_from(command)?)),
            Command::Viewer { command } =>
                Ok(Request::Viewer(viewer::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Api(inner) => inner.into_command(),
            Request::Db(inner) => inner.into_command(),
            Request::Mcp(inner) => inner.into_command(),
            Request::Viewer(inner) => inner.into_command(),
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
            Request::Api(req) => {
                let inner = api::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Api)))
            }
            Request::Db(req) => {
                let inner = db::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Db)))
            }
            Request::Mcp(req) => {
                let inner = mcp::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
            }
            Request::Viewer(req) => {
                let inner = viewer::execute(executor, req, agent_arguments).await?;
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

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Api(req) => {
                let inner = api::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Db(req) => {
                let inner = db::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Mcp(req) => {
                let inner = mcp::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Viewer(req) => {
                let inner = viewer::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
