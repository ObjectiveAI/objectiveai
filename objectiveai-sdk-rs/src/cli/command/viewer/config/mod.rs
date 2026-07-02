pub mod address;
pub mod get;
pub mod secret;
pub mod signature;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Address {
        #[command(subcommand)]
        command: address::Command,
    },
    Secret {
        #[command(subcommand)]
        command: secret::Command,
    },
    Signature {
        #[command(subcommand)]
        command: signature::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.viewer.config.Request")]
pub enum Request {
    #[schemars(title = "Get")]
    Get(get::Request),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Request),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Request),
    #[schemars(title = "Address")]
    Address(address::Request),
    #[schemars(title = "Secret")]
    Secret(secret::Request),
    #[schemars(title = "Signature")]
    Signature(signature::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.viewer.config.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Get")]
    Get(get::Response),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Response),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Response),
    #[schemars(title = "Address")]
    Address(address::Response),
    #[schemars(title = "Secret")]
    Secret(secret::Response),
    #[schemars(title = "Signature")]
    Signature(signature::Response),
}

/// Viewer-stream mirror of [`Request`] — the real command requests only
/// (schema-introspection variants are excluded; the viewer streams
/// actual command traffic). Untagged: each variant carries the leaf's
/// `path_type`, so it stays discriminable.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.viewer.config.ViewerRequest")]
pub enum ViewerRequest {
    #[schemars(title = "Get")]
    Get(get::ViewerRequest),
    #[schemars(title = "Address")]
    Address(address::ViewerRequest),
    #[schemars(title = "Secret")]
    Secret(secret::ViewerRequest),
    #[schemars(title = "Signature")]
    Signature(signature::ViewerRequest),
}

/// Viewer-stream mirror of [`Response`] — mirrors the base response
/// aggregate: unary children carry their `ViewerResponse`, streaming
/// children their `ViewerResponseItem`. Exempt from json-schema coverage:
/// untagged response aggregate (mirrors the base `Response`, TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.viewer.config.ViewerResponseItem")]
pub enum ViewerResponseItem {
    #[schemars(title = "Get")]
    Get(get::ViewerResponse),
    #[schemars(title = "Address")]
    Address(address::ViewerResponseItem),
    #[schemars(title = "Secret")]
    Secret(secret::ViewerResponseItem),
    #[schemars(title = "Signature")]
    Signature(signature::ViewerResponseItem),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Get(v) => v.into_mcp(),
            Response::GetRequestSchema(v) => v.into_mcp(),
            Response::GetResponseSchema(v) => v.into_mcp(),
            Response::Address(v) => v.into_mcp(),
            Response::Secret(v) => v.into_mcp(),
            Response::Signature(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
            Command::Address { command } =>
                Ok(Request::Address(address::Request::try_from(command)?)),
            Command::Secret { command } =>
                Ok(Request::Secret(secret::Request::try_from(command)?)),
            Command::Signature { command } =>
                Ok(Request::Signature(signature::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Get(inner) => inner.request_base(),
            Request::GetRequestSchema(inner) => inner.request_base(),
            Request::GetResponseSchema(inner) => inner.request_base(),
            Request::Address(inner) => inner.request_base(),
            Request::Secret(inner) => inner.request_base(),
            Request::Signature(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Get(inner) => inner.request_base_mut(),
            Request::GetRequestSchema(inner) => inner.request_base_mut(),
            Request::GetResponseSchema(inner) => inner.request_base_mut(),
            Request::Address(inner) => inner.request_base_mut(),
            Request::Secret(inner) => inner.request_base_mut(),
            Request::Signature(inner) => inner.request_base_mut(),
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
            Request::Get(req) => {
                let value = get::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Get(value),
                )))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GetRequestSchema(value),
                )))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GetResponseSchema(value),
                )))
            }
            Request::Address(req) => {
                let inner = address::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Address)))
            }
            Request::Secret(req) => {
                let inner = secret::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Secret)))
            }
            Request::Signature(req) => {
                let inner = signature::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Signature)))
            }
        };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Get(req) => {
                let value = get::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Address(req) => {
                let inner = address::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Secret(req) => {
                let inner = secret::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Signature(req) => {
                let inner = signature::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
