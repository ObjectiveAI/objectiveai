pub mod address;
pub mod database;
pub mod get;
pub mod password;
pub mod user;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Address {
        #[command(subcommand)]
        command: address::Command,
    },
    User {
        #[command(subcommand)]
        command: user::Command,
    },
    Password {
        #[command(subcommand)]
        command: password::Command,
    },
    Database {
        #[command(subcommand)]
        command: database::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.db.config.Request")]
pub enum Request {
    #[schemars(title = "Get")]
    Get(get::Request),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Request),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Request),
    #[schemars(title = "Address")]
    Address(address::Request),
    #[schemars(title = "User")]
    User(user::Request),
    #[schemars(title = "Password")]
    Password(password::Request),
    #[schemars(title = "Database")]
    Database(database::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.db.config.Response")]
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
    #[schemars(title = "User")]
    User(user::Response),
    #[schemars(title = "Password")]
    Password(password::Response),
    #[schemars(title = "Database")]
    Database(database::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Get(v) => v.into_mcp(),
            Response::GetRequestSchema(v) => v.into_mcp(),
            Response::GetResponseSchema(v) => v.into_mcp(),
            Response::Address(v) => v.into_mcp(),
            Response::User(v) => v.into_mcp(),
            Response::Password(v) => v.into_mcp(),
            Response::Database(v) => v.into_mcp(),
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
            Command::User { command } =>
                Ok(Request::User(user::Request::try_from(command)?)),
            Command::Password { command } =>
                Ok(Request::Password(password::Request::try_from(command)?)),
            Command::Database { command } =>
                Ok(Request::Database(database::Request::try_from(command)?)),
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
            Request::User(inner) => inner.request_base(),
            Request::Password(inner) => inner.request_base(),
            Request::Database(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Get(inner) => inner.request_base_mut(),
            Request::GetRequestSchema(inner) => inner.request_base_mut(),
            Request::GetResponseSchema(inner) => inner.request_base_mut(),
            Request::Address(inner) => inner.request_base_mut(),
            Request::User(inner) => inner.request_base_mut(),
            Request::Password(inner) => inner.request_base_mut(),
            Request::Database(inner) => inner.request_base_mut(),
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
            Request::User(req) => {
                let inner = user::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::User)))
            }
            Request::Password(req) => {
                let inner = password::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Password)))
            }
            Request::Database(req) => {
                let inner = database::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Database)))
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
            Request::User(req) => {
                let inner = user::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Password(req) => {
                let inner = password::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Database(req) => {
                let inner = database::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Get(get::ListenerExecution),
    GetRequestSchema(get::request_schema::ListenerExecution),
    GetResponseSchema(get::response_schema::ListenerExecution),
    Address(address::ListenerExecution),
    User(user::ListenerExecution),
    Password(password::ListenerExecution),
    Database(database::ListenerExecution),
}
