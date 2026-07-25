//! `agents mcp` — query a live agent's aggregated MCP surface over its
//! per-`response_id` listener socket. Sub-groups:
//!
//! - `resources` — `list`, `read`.
//! - `servers` — `list`.
//! - `tools` — `call`, `list`.

use crate::cli::command::CommandRequest;

pub mod resources;
pub mod servers;
pub mod tools;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Query a live agent's MCP resources — `list`, `read` — over its
    /// per-`response_id` listener socket.
    Resources {
        #[command(subcommand)]
        command: resources::Command,
    },
    /// List a live agent's connected MCP servers and their metadata.
    Servers {
        #[command(subcommand)]
        command: servers::Command,
    },
    /// Query a live agent's MCP tools — `call`, `list` — over its
    /// per-`response_id` listener socket.
    Tools {
        #[command(subcommand)]
        command: tools::Command,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.mcp.Request")]
pub enum Request {
    #[schemars(title = "Resources")]
    Resources(resources::Request),
    #[schemars(title = "Servers")]
    Servers(servers::Request),
    #[schemars(title = "Tools")]
    Tools(tools::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.mcp.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Resources")]
    Resources(resources::ResponseItem),
    #[schemars(title = "Servers")]
    Servers(servers::ResponseItem),
    #[schemars(title = "Tools")]
    Tools(tools::ResponseItem),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Resources(v) => v.into_mcp(),
            ResponseItem::Servers(v) => v.into_mcp(),
            ResponseItem::Tools(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Resources { command } => {
                Ok(Request::Resources(resources::Request::try_from(command)?))
            }
            Command::Servers { command } => {
                Ok(Request::Servers(servers::Request::try_from(command)?))
            }
            Command::Tools { command } => {
                Ok(Request::Tools(tools::Request::try_from(command)?))
            }
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Resources(inner) => inner.request_base(),
            Request::Servers(inner) => inner.request_base(),
            Request::Tools(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Resources(inner) => inner.request_base_mut(),
            Request::Servers(inner) => inner.request_base_mut(),
            Request::Tools(inner) => inner.request_base_mut(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    identity: Option<&crate::identity::Identity>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
        Request::Resources(req) => {
            let inner = resources::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Resources)))
        }
        Request::Servers(req) => {
            let inner = servers::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Servers)))
        }
        Request::Tools(req) => {
            let inner = tools::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Tools)))
        }
    };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    transform: crate::cli::command::Transform,
    identity: Option<&crate::identity::Identity>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>,
    > = match request {
        Request::Resources(req) => {
            let inner = resources::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
        Request::Servers(req) => {
            let inner = servers::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
        Request::Tools(req) => {
            let inner = tools::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Resources(resources::ListenerExecution),
    Servers(servers::ListenerExecution),
    Tools(tools::ListenerExecution),
}
