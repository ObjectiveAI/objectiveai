pub mod address;
pub mod get;
pub mod refresh_secret_signature_pair;
pub mod secret;
pub mod set;
pub mod signature;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Set(set::Command),
    Address {
        #[command(subcommand)]
        command: address::Command,
    },
    RefreshSecretSignaturePair(refresh_secret_signature_pair::Command),
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
#[schemars(rename = "cli.command.daemon.config.Request")]
pub enum Request {
    #[schemars(title = "Get")]
    Get(get::Request),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Request),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Request),
    #[schemars(title = "Set")]
    Set(set::Request),
    #[schemars(title = "SetRequestSchema")]
    SetRequestSchema(set::request_schema::Request),
    #[schemars(title = "SetResponseSchema")]
    SetResponseSchema(set::response_schema::Request),
    #[schemars(title = "Address")]
    Address(address::Request),
    #[schemars(title = "RefreshSecretSignaturePair")]
    RefreshSecretSignaturePair(refresh_secret_signature_pair::Request),
    #[schemars(title = "RefreshSecretSignaturePairRequestSchema")]
    RefreshSecretSignaturePairRequestSchema(refresh_secret_signature_pair::request_schema::Request),
    #[schemars(title = "RefreshSecretSignaturePairResponseSchema")]
    RefreshSecretSignaturePairResponseSchema(refresh_secret_signature_pair::response_schema::Request),
    #[schemars(title = "Secret")]
    Secret(secret::Request),
    #[schemars(title = "Signature")]
    Signature(signature::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.daemon.config.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Get")]
    Get(get::Response),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Response),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Response),
    #[schemars(title = "Set")]
    Set(set::Response),
    #[schemars(title = "SetRequestSchema")]
    SetRequestSchema(set::request_schema::Response),
    #[schemars(title = "SetResponseSchema")]
    SetResponseSchema(set::response_schema::Response),
    #[schemars(title = "Address")]
    Address(address::Response),
    #[schemars(title = "RefreshSecretSignaturePair")]
    RefreshSecretSignaturePair(refresh_secret_signature_pair::Response),
    #[schemars(title = "RefreshSecretSignaturePairRequestSchema")]
    RefreshSecretSignaturePairRequestSchema(refresh_secret_signature_pair::request_schema::Response),
    #[schemars(title = "RefreshSecretSignaturePairResponseSchema")]
    RefreshSecretSignaturePairResponseSchema(refresh_secret_signature_pair::response_schema::Response),
    #[schemars(title = "Secret")]
    Secret(secret::Response),
    #[schemars(title = "Signature")]
    Signature(signature::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Get(v) => v.into_mcp(),
            Response::GetRequestSchema(v) => v.into_mcp(),
            Response::GetResponseSchema(v) => v.into_mcp(),
            Response::Set(v) => v.into_mcp(),
            Response::SetRequestSchema(v) => v.into_mcp(),
            Response::SetResponseSchema(v) => v.into_mcp(),
            Response::Address(v) => v.into_mcp(),
            Response::RefreshSecretSignaturePair(v) => v.into_mcp(),
            Response::RefreshSecretSignaturePairRequestSchema(v) => v.into_mcp(),
            Response::RefreshSecretSignaturePairResponseSchema(v) => v.into_mcp(),
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
            Command::Set(cmd) => match cmd.schema {
                None => Ok(Request::Set(set::Request::try_from(cmd.args)?)),
                Some(set::Schema::RequestSchema(args)) =>
                    Ok(Request::SetRequestSchema(set::request_schema::Request::try_from(args)?)),
                Some(set::Schema::ResponseSchema(args)) =>
                    Ok(Request::SetResponseSchema(set::response_schema::Request::try_from(args)?)),
            },
            Command::Address { command } =>
                Ok(Request::Address(address::Request::try_from(command)?)),
            Command::RefreshSecretSignaturePair(cmd) => match cmd.schema {
                None => Ok(Request::RefreshSecretSignaturePair(
                    refresh_secret_signature_pair::Request::try_from(cmd.args)?,
                )),
                Some(refresh_secret_signature_pair::Schema::RequestSchema(args)) =>
                    Ok(Request::RefreshSecretSignaturePairRequestSchema(
                        refresh_secret_signature_pair::request_schema::Request::try_from(args)?,
                    )),
                Some(refresh_secret_signature_pair::Schema::ResponseSchema(args)) =>
                    Ok(Request::RefreshSecretSignaturePairResponseSchema(
                        refresh_secret_signature_pair::response_schema::Request::try_from(args)?,
                    )),
            },
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
            Request::Set(inner) => inner.request_base(),
            Request::SetRequestSchema(inner) => inner.request_base(),
            Request::SetResponseSchema(inner) => inner.request_base(),
            Request::Address(inner) => inner.request_base(),
            Request::RefreshSecretSignaturePair(inner) => inner.request_base(),
            Request::RefreshSecretSignaturePairRequestSchema(inner) => inner.request_base(),
            Request::RefreshSecretSignaturePairResponseSchema(inner) => inner.request_base(),
            Request::Secret(inner) => inner.request_base(),
            Request::Signature(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Get(inner) => inner.request_base_mut(),
            Request::GetRequestSchema(inner) => inner.request_base_mut(),
            Request::GetResponseSchema(inner) => inner.request_base_mut(),
            Request::Set(inner) => inner.request_base_mut(),
            Request::SetRequestSchema(inner) => inner.request_base_mut(),
            Request::SetResponseSchema(inner) => inner.request_base_mut(),
            Request::Address(inner) => inner.request_base_mut(),
            Request::RefreshSecretSignaturePair(inner) => inner.request_base_mut(),
            Request::RefreshSecretSignaturePairRequestSchema(inner) => inner.request_base_mut(),
            Request::RefreshSecretSignaturePairResponseSchema(inner) => inner.request_base_mut(),
            Request::Secret(inner) => inner.request_base_mut(),
            Request::Signature(inner) => inner.request_base_mut(),
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
            Request::Get(req) => {
                let value = get::execute(executor, req, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Get(value),
                )))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute(executor, req, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GetRequestSchema(value),
                )))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute(executor, req, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GetResponseSchema(value),
                )))
            }
            Request::Set(req) => {
                let value = set::execute(executor, req, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Set(value),
                )))
            }
            Request::SetRequestSchema(req) => {
                let value = set::request_schema::execute(executor, req, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::SetRequestSchema(value),
                )))
            }
            Request::SetResponseSchema(req) => {
                let value = set::response_schema::execute(executor, req, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::SetResponseSchema(value),
                )))
            }
            Request::Address(req) => {
                let inner = address::execute(executor, req, identity).await?;
                Box::pin(inner.map(|r| r.map(Response::Address)))
            }
            Request::RefreshSecretSignaturePair(req) => {
                let value =
                    refresh_secret_signature_pair::execute(executor, req, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::RefreshSecretSignaturePair(value),
                )))
            }
            Request::RefreshSecretSignaturePairRequestSchema(req) => {
                let value = refresh_secret_signature_pair::request_schema::execute(
                    executor, req, identity,
                )
                .await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::RefreshSecretSignaturePairRequestSchema(value),
                )))
            }
            Request::RefreshSecretSignaturePairResponseSchema(req) => {
                let value = refresh_secret_signature_pair::response_schema::execute(
                    executor, req, identity,
                )
                .await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::RefreshSecretSignaturePairResponseSchema(value),
                )))
            }
            Request::Secret(req) => {
                let inner = secret::execute(executor, req, identity).await?;
                Box::pin(inner.map(|r| r.map(Response::Secret)))
            }
            Request::Signature(req) => {
                let inner = signature::execute(executor, req, identity).await?;
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

        identity: Option<&crate::identity::Identity>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Get(req) => {
                let value = get::execute_transform(executor, req, transform, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute_transform(executor, req, transform, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute_transform(executor, req, transform, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Set(req) => {
                let value = set::execute_transform(executor, req, transform, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SetRequestSchema(req) => {
                let value = set::request_schema::execute_transform(executor, req, transform, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SetResponseSchema(req) => {
                let value = set::response_schema::execute_transform(executor, req, transform, identity).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Address(req) => {
                let inner = address::execute_transform(executor, req, transform, identity).await?;
                Box::pin(inner)
            }
            Request::RefreshSecretSignaturePair(req) => {
                let value = refresh_secret_signature_pair::execute_transform(
                    executor, req, transform, identity,
                )
                .await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::RefreshSecretSignaturePairRequestSchema(req) => {
                let value = refresh_secret_signature_pair::request_schema::execute_transform(
                    executor, req, transform, identity,
                )
                .await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::RefreshSecretSignaturePairResponseSchema(req) => {
                let value = refresh_secret_signature_pair::response_schema::execute_transform(
                    executor, req, transform, identity,
                )
                .await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Secret(req) => {
                let inner = secret::execute_transform(executor, req, transform, identity).await?;
                Box::pin(inner)
            }
            Request::Signature(req) => {
                let inner = signature::execute_transform(executor, req, transform, identity).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Get(get::ListenerExecution),
    GetRequestSchema(get::request_schema::ListenerExecution),
    GetResponseSchema(get::response_schema::ListenerExecution),
    Set(set::ListenerExecution),
    SetRequestSchema(set::request_schema::ListenerExecution),
    SetResponseSchema(set::response_schema::ListenerExecution),
    Address(address::ListenerExecution),
    RefreshSecretSignaturePair(refresh_secret_signature_pair::ListenerExecution),
    RefreshSecretSignaturePairRequestSchema(refresh_secret_signature_pair::request_schema::ListenerExecution),
    RefreshSecretSignaturePairResponseSchema(refresh_secret_signature_pair::response_schema::ListenerExecution),
    Secret(secret::ListenerExecution),
    Signature(signature::ListenerExecution),
}
