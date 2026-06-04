pub mod alpha_scalar;
pub mod alpha_vector;
pub mod remote;

#[derive(clap::Subcommand)]
pub enum Command {
    AlphaScalar(alpha_scalar::Command),
    AlphaVector(alpha_vector::Command),
    Remote(remote::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.functions.inventions.recursive.create.Request")]
pub enum Request {
    #[schemars(title = "AlphaScalar")]
    AlphaScalar(alpha_scalar::Request),
    #[schemars(title = "AlphaScalarRequestSchema")]
    AlphaScalarRequestSchema(alpha_scalar::request_schema::Request),
    #[schemars(title = "AlphaScalarResponseSchema")]
    AlphaScalarResponseSchema(alpha_scalar::response_schema::Request),
    #[schemars(title = "AlphaVector")]
    AlphaVector(alpha_vector::Request),
    #[schemars(title = "AlphaVectorRequestSchema")]
    AlphaVectorRequestSchema(alpha_vector::request_schema::Request),
    #[schemars(title = "AlphaVectorResponseSchema")]
    AlphaVectorResponseSchema(alpha_vector::response_schema::Request),
    #[schemars(title = "Remote")]
    Remote(remote::Request),
    #[schemars(title = "RemoteRequestSchema")]
    RemoteRequestSchema(remote::request_schema::Request),
    #[schemars(title = "RemoteResponseSchema")]
    RemoteResponseSchema(remote::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.inventions.recursive.create.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "AlphaScalar")]
    AlphaScalar(alpha_scalar::ResponseItem),
    #[schemars(title = "AlphaScalarRequestSchema")]
    AlphaScalarRequestSchema(alpha_scalar::request_schema::Response),
    #[schemars(title = "AlphaScalarResponseSchema")]
    AlphaScalarResponseSchema(alpha_scalar::response_schema::Response),
    #[schemars(title = "AlphaVector")]
    AlphaVector(alpha_vector::ResponseItem),
    #[schemars(title = "AlphaVectorRequestSchema")]
    AlphaVectorRequestSchema(alpha_vector::request_schema::Response),
    #[schemars(title = "AlphaVectorResponseSchema")]
    AlphaVectorResponseSchema(alpha_vector::response_schema::Response),
    #[schemars(title = "Remote")]
    Remote(remote::ResponseItem),
    #[schemars(title = "RemoteRequestSchema")]
    RemoteRequestSchema(remote::request_schema::Response),
    #[schemars(title = "RemoteResponseSchema")]
    RemoteResponseSchema(remote::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::AlphaScalar(v) => v.into_mcp(),
            ResponseItem::AlphaScalarRequestSchema(v) => v.into_mcp(),
            ResponseItem::AlphaScalarResponseSchema(v) => v.into_mcp(),
            ResponseItem::AlphaVector(v) => v.into_mcp(),
            ResponseItem::AlphaVectorRequestSchema(v) => v.into_mcp(),
            ResponseItem::AlphaVectorResponseSchema(v) => v.into_mcp(),
            ResponseItem::Remote(v) => v.into_mcp(),
            ResponseItem::RemoteRequestSchema(v) => v.into_mcp(),
            ResponseItem::RemoteResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::AlphaScalar(cmd) => match cmd.schema {
                None => Ok(Request::AlphaScalar(alpha_scalar::Request::try_from(cmd.args)?)),
                Some(alpha_scalar::Schema::RequestSchema(args)) =>
                    Ok(Request::AlphaScalarRequestSchema(alpha_scalar::request_schema::Request::try_from(args)?)),
                Some(alpha_scalar::Schema::ResponseSchema(args)) =>
                    Ok(Request::AlphaScalarResponseSchema(alpha_scalar::response_schema::Request::try_from(args)?)),
            },
            Command::AlphaVector(cmd) => match cmd.schema {
                None => Ok(Request::AlphaVector(alpha_vector::Request::try_from(cmd.args)?)),
                Some(alpha_vector::Schema::RequestSchema(args)) =>
                    Ok(Request::AlphaVectorRequestSchema(alpha_vector::request_schema::Request::try_from(args)?)),
                Some(alpha_vector::Schema::ResponseSchema(args)) =>
                    Ok(Request::AlphaVectorResponseSchema(alpha_vector::response_schema::Request::try_from(args)?)),
            },
            Command::Remote(cmd) => match cmd.schema {
                None => Ok(Request::Remote(remote::Request::try_from(cmd.args)?)),
                Some(remote::Schema::RequestSchema(args)) =>
                    Ok(Request::RemoteRequestSchema(remote::request_schema::Request::try_from(args)?)),
                Some(remote::Schema::ResponseSchema(args)) =>
                    Ok(Request::RemoteResponseSchema(remote::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::AlphaScalar(inner) => inner.into_command(),
            Request::AlphaScalarRequestSchema(inner) => inner.into_command(),
            Request::AlphaScalarResponseSchema(inner) => inner.into_command(),
            Request::AlphaVector(inner) => inner.into_command(),
            Request::AlphaVectorRequestSchema(inner) => inner.into_command(),
            Request::AlphaVectorResponseSchema(inner) => inner.into_command(),
            Request::Remote(inner) => inner.into_command(),
            Request::RemoteRequestSchema(inner) => inner.into_command(),
            Request::RemoteResponseSchema(inner) => inner.into_command(),
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
            Request::AlphaScalar(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = alpha_scalar::execute_streaming(executor, req, agent_arguments).await?;
                    Box::pin(inner.map(|r| r.map(ResponseItem::AlphaScalar)))
                } else {
                    let value = alpha_scalar::execute(executor, req, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(
                        ResponseItem::AlphaScalar(alpha_scalar::ResponseItem::Id(value)),
                    )))
                }
            }
            Request::AlphaScalarRequestSchema(req) => {
                let value = alpha_scalar::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::AlphaScalarRequestSchema(value),
                )))
            }
            Request::AlphaScalarResponseSchema(req) => {
                let value = alpha_scalar::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::AlphaScalarResponseSchema(value),
                )))
            }
            Request::AlphaVector(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = alpha_vector::execute_streaming(executor, req, agent_arguments).await?;
                    Box::pin(inner.map(|r| r.map(ResponseItem::AlphaVector)))
                } else {
                    let value = alpha_vector::execute(executor, req, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(
                        ResponseItem::AlphaVector(alpha_vector::ResponseItem::Id(value)),
                    )))
                }
            }
            Request::AlphaVectorRequestSchema(req) => {
                let value = alpha_vector::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::AlphaVectorRequestSchema(value),
                )))
            }
            Request::AlphaVectorResponseSchema(req) => {
                let value = alpha_vector::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::AlphaVectorResponseSchema(value),
                )))
            }
            Request::Remote(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = remote::execute_streaming(executor, req, agent_arguments).await?;
                    Box::pin(inner.map(|r| r.map(ResponseItem::Remote)))
                } else {
                    let value = remote::execute(executor, req, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(
                        ResponseItem::Remote(remote::ResponseItem::Id(value)),
                    )))
                }
            }
            Request::RemoteRequestSchema(req) => {
                let value = remote::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::RemoteRequestSchema(value),
                )))
            }
            Request::RemoteResponseSchema(req) => {
                let value = remote::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::RemoteResponseSchema(value),
                )))
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
            Request::AlphaScalar(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = alpha_scalar::execute_streaming_jq(executor, req, jq, agent_arguments).await?;
                    Box::pin(inner)
                } else {
                    let value = alpha_scalar::execute_jq(executor, req, jq, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
                }
            }
            Request::AlphaScalarRequestSchema(req) => {
                let value = alpha_scalar::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::AlphaScalarResponseSchema(req) => {
                let value = alpha_scalar::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::AlphaVector(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = alpha_vector::execute_streaming_jq(executor, req, jq, agent_arguments).await?;
                    Box::pin(inner)
                } else {
                    let value = alpha_vector::execute_jq(executor, req, jq, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
                }
            }
            Request::AlphaVectorRequestSchema(req) => {
                let value = alpha_vector::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::AlphaVectorResponseSchema(req) => {
                let value = alpha_vector::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Remote(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = remote::execute_streaming_jq(executor, req, jq, agent_arguments).await?;
                    Box::pin(inner)
                } else {
                    let value = remote::execute_jq(executor, req, jq, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
                }
            }
            Request::RemoteRequestSchema(req) => {
                let value = remote::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::RemoteResponseSchema(req) => {
                let value = remote::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
