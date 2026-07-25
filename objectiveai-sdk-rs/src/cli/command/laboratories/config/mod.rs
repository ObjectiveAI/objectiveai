pub mod addresses;
pub mod local;

#[derive(clap::Subcommand)]
pub enum Command {
    /// The daemon addresses the laboratory host connects to, each with
    /// an optional signature.
    Addresses {
        #[command(subcommand)]
        command: addresses::Command,
    },
    /// Whether the host connects to the LOCAL daemon on spawn
    /// (default true).
    Local {
        #[command(subcommand)]
        command: local::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.laboratories.config.Request")]
pub enum Request {
    #[schemars(title = "Addresses")]
    Addresses(addresses::Request),
    #[schemars(title = "Local")]
    Local(local::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.config.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Addresses")]
    Addresses(addresses::Response),
    #[schemars(title = "Local")]
    Local(local::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Addresses(v) => v.into_mcp(),
            Response::Local(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Addresses { command } => {
                Ok(Request::Addresses(addresses::Request::try_from(command)?))
            }
            Command::Local { command } => {
                Ok(Request::Local(local::Request::try_from(command)?))
            }
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Addresses(inner) => inner.request_base(),
            Request::Local(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Addresses(inner) => inner.request_base_mut(),
            Request::Local(inner) => inner.request_base_mut(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,

        identity: Option<&crate::identity::Identity>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>> =
        match request {
            Request::Addresses(req) => {
                let inner = addresses::execute(executor, req, identity).await?;
                Box::pin(inner.map(|r| r.map(Response::Addresses)))
            }
            Request::Local(req) => {
                let inner = local::execute(executor, req, identity).await?;
                Box::pin(inner.map(|r| r.map(Response::Local)))
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
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Addresses(req) => {
                let inner = addresses::execute_transform(executor, req, transform, identity).await?;
                Box::pin(inner)
            }
            Request::Local(req) => {
                let inner = local::execute_transform(executor, req, transform, identity).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Addresses(addresses::ListenerExecution),
    Local(local::ListenerExecution),
}
