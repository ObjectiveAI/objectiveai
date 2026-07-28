//! `development plugins` — the development-mode registrations for
//! plugins, split the way the manifest itself splits: `mcp` (the
//! laboratory builds the server from a local directory) and `viewer`
//! (the viewer serves assets live from one). A plugin's two halves
//! register independently.

use crate::cli::command::CommandRequest;

pub mod mcp;
pub mod viewer;

#[derive(clap::Subcommand)]
pub enum Command {
    /// The MCP-server half.
    Mcp {
        #[command(subcommand)]
        command: mcp::Command,
    },
    /// The viewer half.
    Viewer {
        #[command(subcommand)]
        command: viewer::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.development.plugins.Request")]
pub enum Request {
    #[schemars(title = "Mcp")]
    Mcp(mcp::Request),
    #[schemars(title = "Viewer")]
    Viewer(viewer::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.plugins.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Mcp")]
    Mcp(mcp::ResponseItem),
    #[schemars(title = "Viewer")]
    Viewer(viewer::ResponseItem),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Mcp(v) => v.into_mcp(),
            ResponseItem::Viewer(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Mcp { command } => Ok(Request::Mcp(mcp::Request::try_from(command)?)),
            Command::Viewer { command } => {
                Ok(Request::Viewer(viewer::Request::try_from(command)?))
            }
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Mcp(inner) => inner.request_base(),
            Request::Viewer(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Mcp(inner) => inner.request_base_mut(),
            Request::Viewer(inner) => inner.request_base_mut(),
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
        Request::Mcp(req) => {
            let inner = mcp::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
        }
        Request::Viewer(req) => {
            let inner = viewer::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Viewer)))
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
        Request::Mcp(req) => {
            let inner = mcp::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
        Request::Viewer(req) => {
            let inner = viewer::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub enum ListenerExecution {
    Mcp(mcp::ListenerExecution),
    Viewer(viewer::ListenerExecution),
}
