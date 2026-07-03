pub mod config;
pub mod generate_secret_signature_pair;
pub mod kill;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    Config {
        #[command(subcommand)]
        command: config::Command,
    },
    GenerateSecretSignaturePair(generate_secret_signature_pair::Command),
    Kill(kill::Command),
    Spawn(spawn::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.viewer.Request")]
pub enum Request {
    #[schemars(title = "Config")]
    Config(config::Request),
    #[schemars(title = "GenerateSecretSignaturePair")]
    GenerateSecretSignaturePair(generate_secret_signature_pair::Request),
    #[schemars(title = "GenerateSecretSignaturePairRequestSchema")]
    GenerateSecretSignaturePairRequestSchema(generate_secret_signature_pair::request_schema::Request),
    #[schemars(title = "GenerateSecretSignaturePairResponseSchema")]
    GenerateSecretSignaturePairResponseSchema(generate_secret_signature_pair::response_schema::Request),
    #[schemars(title = "Kill")]
    Kill(kill::Request),
    #[schemars(title = "KillRequestSchema")]
    KillRequestSchema(kill::request_schema::Request),
    #[schemars(title = "KillResponseSchema")]
    KillResponseSchema(kill::response_schema::Request),
    #[schemars(title = "Spawn")]
    Spawn(spawn::Request),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Request),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.viewer.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Config")]
    Config(config::Response),
    #[schemars(title = "GenerateSecretSignaturePair")]
    GenerateSecretSignaturePair(generate_secret_signature_pair::Response),
    #[schemars(title = "GenerateSecretSignaturePairRequestSchema")]
    GenerateSecretSignaturePairRequestSchema(generate_secret_signature_pair::request_schema::Response),
    #[schemars(title = "GenerateSecretSignaturePairResponseSchema")]
    GenerateSecretSignaturePairResponseSchema(generate_secret_signature_pair::response_schema::Response),
    #[schemars(title = "Kill")]
    Kill(kill::Response),
    #[schemars(title = "KillRequestSchema")]
    KillRequestSchema(kill::request_schema::Response),
    #[schemars(title = "KillResponseSchema")]
    KillResponseSchema(kill::response_schema::Response),
    #[schemars(title = "Spawn")]
    Spawn(spawn::Response),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Response),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Config(v) => v.into_mcp(),
            Response::GenerateSecretSignaturePair(v) => v.into_mcp(),
            Response::GenerateSecretSignaturePairRequestSchema(v) => v.into_mcp(),
            Response::GenerateSecretSignaturePairResponseSchema(v) => v.into_mcp(),
            Response::Kill(v) => v.into_mcp(),
            Response::KillRequestSchema(v) => v.into_mcp(),
            Response::KillResponseSchema(v) => v.into_mcp(),
            Response::Spawn(v) => v.into_mcp(),
            Response::SpawnRequestSchema(v) => v.into_mcp(),
            Response::SpawnResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Config { command } =>
                Ok(Request::Config(config::Request::try_from(command)?)),
            Command::GenerateSecretSignaturePair(cmd) => match cmd.schema {
                None => Ok(Request::GenerateSecretSignaturePair(generate_secret_signature_pair::Request::try_from(cmd.args)?)),
                Some(generate_secret_signature_pair::Schema::RequestSchema(args)) =>
                    Ok(Request::GenerateSecretSignaturePairRequestSchema(generate_secret_signature_pair::request_schema::Request::try_from(args)?)),
                Some(generate_secret_signature_pair::Schema::ResponseSchema(args)) =>
                    Ok(Request::GenerateSecretSignaturePairResponseSchema(generate_secret_signature_pair::response_schema::Request::try_from(args)?)),
            },
            Command::Kill(cmd) => match cmd.schema {
                None => Ok(Request::Kill(kill::Request::try_from(cmd.args)?)),
                Some(kill::Schema::RequestSchema(args)) =>
                    Ok(Request::KillRequestSchema(kill::request_schema::Request::try_from(args)?)),
                Some(kill::Schema::ResponseSchema(args)) =>
                    Ok(Request::KillResponseSchema(kill::response_schema::Request::try_from(args)?)),
            },
            Command::Spawn(cmd) => match cmd.schema {
                None => Ok(Request::Spawn(spawn::Request::try_from(cmd.args)?)),
                Some(spawn::Schema::RequestSchema(args)) =>
                    Ok(Request::SpawnRequestSchema(spawn::request_schema::Request::try_from(args)?)),
                Some(spawn::Schema::ResponseSchema(args)) =>
                    Ok(Request::SpawnResponseSchema(spawn::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Config(inner) => inner.request_base(),
            Request::GenerateSecretSignaturePair(inner) => inner.request_base(),
            Request::GenerateSecretSignaturePairRequestSchema(inner) => inner.request_base(),
            Request::GenerateSecretSignaturePairResponseSchema(inner) => inner.request_base(),
            Request::Kill(inner) => inner.request_base(),
            Request::KillRequestSchema(inner) => inner.request_base(),
            Request::KillResponseSchema(inner) => inner.request_base(),
            Request::Spawn(inner) => inner.request_base(),
            Request::SpawnRequestSchema(inner) => inner.request_base(),
            Request::SpawnResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Config(inner) => inner.request_base_mut(),
            Request::GenerateSecretSignaturePair(inner) => inner.request_base_mut(),
            Request::GenerateSecretSignaturePairRequestSchema(inner) => inner.request_base_mut(),
            Request::GenerateSecretSignaturePairResponseSchema(inner) => inner.request_base_mut(),
            Request::Kill(inner) => inner.request_base_mut(),
            Request::KillRequestSchema(inner) => inner.request_base_mut(),
            Request::KillResponseSchema(inner) => inner.request_base_mut(),
            Request::Spawn(inner) => inner.request_base_mut(),
            Request::SpawnRequestSchema(inner) => inner.request_base_mut(),
            Request::SpawnResponseSchema(inner) => inner.request_base_mut(),
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
            Request::Config(req) => {
                let inner = config::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Config)))
            }
            Request::GenerateSecretSignaturePair(req) => {
                let value = generate_secret_signature_pair::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GenerateSecretSignaturePair(value),
                )))
            }
            Request::GenerateSecretSignaturePairRequestSchema(req) => {
                let value = generate_secret_signature_pair::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GenerateSecretSignaturePairRequestSchema(value),
                )))
            }
            Request::GenerateSecretSignaturePairResponseSchema(req) => {
                let value = generate_secret_signature_pair::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GenerateSecretSignaturePairResponseSchema(value),
                )))
            }
            Request::Kill(req) => {
                let value = kill::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Kill(value),
                )))
            }
            Request::KillRequestSchema(req) => {
                let value = kill::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::KillRequestSchema(value),
                )))
            }
            Request::KillResponseSchema(req) => {
                let value = kill::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::KillResponseSchema(value),
                )))
            }
            Request::Spawn(req) => {
                let value = spawn::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Spawn(value),
                )))
            }
            Request::SpawnRequestSchema(req) => {
                let value = spawn::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::SpawnRequestSchema(value),
                )))
            }
            Request::SpawnResponseSchema(req) => {
                let value = spawn::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::SpawnResponseSchema(value),
                )))
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
            Request::Config(req) => {
                let inner = config::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::GenerateSecretSignaturePair(req) => {
                let value = generate_secret_signature_pair::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GenerateSecretSignaturePairRequestSchema(req) => {
                let value = generate_secret_signature_pair::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GenerateSecretSignaturePairResponseSchema(req) => {
                let value = generate_secret_signature_pair::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Kill(req) => {
                let value = kill::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::KillRequestSchema(req) => {
                let value = kill::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::KillResponseSchema(req) => {
                let value = kill::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Spawn(req) => {
                let value = spawn::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SpawnRequestSchema(req) => {
                let value = spawn::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SpawnResponseSchema(req) => {
                let value = spawn::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Config(config::ListenerExecution),
    GenerateSecretSignaturePair(generate_secret_signature_pair::ListenerExecution),
    GenerateSecretSignaturePairRequestSchema(generate_secret_signature_pair::request_schema::ListenerExecution),
    GenerateSecretSignaturePairResponseSchema(generate_secret_signature_pair::response_schema::ListenerExecution),
    Kill(kill::ListenerExecution),
    KillRequestSchema(kill::request_schema::ListenerExecution),
    KillResponseSchema(kill::response_schema::ListenerExecution),
    Spawn(spawn::ListenerExecution),
    SpawnRequestSchema(spawn::request_schema::ListenerExecution),
    SpawnResponseSchema(spawn::response_schema::ListenerExecution),
}
