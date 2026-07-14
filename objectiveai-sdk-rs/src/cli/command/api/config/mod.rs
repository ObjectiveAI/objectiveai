pub mod address;
pub mod backoff_max_elapsed_time_ms;
pub mod commit_author_email;
pub mod commit_author_name;
pub mod get;
pub mod github_authorization;
pub mod http_referer;
pub mod mcp_authorization;
pub mod mcp_call_timeout_ms;
pub mod mcp_connect_timeout_ms;
pub mod objectiveai_authorization;
pub mod openrouter_authorization;
pub mod user_agent;
pub mod x_title;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Address {
        #[command(subcommand)]
        command: address::Command,
    },
    ObjectiveaiAuthorization {
        #[command(subcommand)]
        command: objectiveai_authorization::Command,
    },
    OpenrouterAuthorization {
        #[command(subcommand)]
        command: openrouter_authorization::Command,
    },
    GithubAuthorization {
        #[command(subcommand)]
        command: github_authorization::Command,
    },
    McpAuthorization {
        #[command(subcommand)]
        command: mcp_authorization::Command,
    },
    McpCallTimeoutMs {
        #[command(subcommand)]
        command: mcp_call_timeout_ms::Command,
    },
    McpConnectTimeoutMs {
        #[command(subcommand)]
        command: mcp_connect_timeout_ms::Command,
    },
    BackoffMaxElapsedTimeMs {
        #[command(subcommand)]
        command: backoff_max_elapsed_time_ms::Command,
    },
    UserAgent {
        #[command(subcommand)]
        command: user_agent::Command,
    },
    HttpReferer {
        #[command(subcommand)]
        command: http_referer::Command,
    },
    XTitle {
        #[command(subcommand)]
        command: x_title::Command,
    },
    CommitAuthorName {
        #[command(subcommand)]
        command: commit_author_name::Command,
    },
    CommitAuthorEmail {
        #[command(subcommand)]
        command: commit_author_email::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.api.config.Request")]
pub enum Request {
    #[schemars(title = "Get")]
    Get(get::Request),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Request),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Request),
    #[schemars(title = "Address")]
    Address(address::Request),
    #[schemars(title = "ObjectiveaiAuthorization")]
    ObjectiveaiAuthorization(objectiveai_authorization::Request),
    #[schemars(title = "OpenrouterAuthorization")]
    OpenrouterAuthorization(openrouter_authorization::Request),
    #[schemars(title = "GithubAuthorization")]
    GithubAuthorization(github_authorization::Request),
    #[schemars(title = "McpAuthorization")]
    McpAuthorization(mcp_authorization::Request),
    #[schemars(title = "McpCallTimeoutMs")]
    McpCallTimeoutMs(mcp_call_timeout_ms::Request),
    #[schemars(title = "McpConnectTimeoutMs")]
    McpConnectTimeoutMs(mcp_connect_timeout_ms::Request),
    #[schemars(title = "BackoffMaxElapsedTimeMs")]
    BackoffMaxElapsedTimeMs(backoff_max_elapsed_time_ms::Request),
    #[schemars(title = "UserAgent")]
    UserAgent(user_agent::Request),
    #[schemars(title = "HttpReferer")]
    HttpReferer(http_referer::Request),
    #[schemars(title = "XTitle")]
    XTitle(x_title::Request),
    #[schemars(title = "CommitAuthorName")]
    CommitAuthorName(commit_author_name::Request),
    #[schemars(title = "CommitAuthorEmail")]
    CommitAuthorEmail(commit_author_email::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.api.config.Response")]
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
    #[schemars(title = "ObjectiveaiAuthorization")]
    ObjectiveaiAuthorization(objectiveai_authorization::Response),
    #[schemars(title = "OpenrouterAuthorization")]
    OpenrouterAuthorization(openrouter_authorization::Response),
    #[schemars(title = "GithubAuthorization")]
    GithubAuthorization(github_authorization::Response),
    #[schemars(title = "McpAuthorization")]
    McpAuthorization(mcp_authorization::Response),
    #[schemars(title = "McpCallTimeoutMs")]
    McpCallTimeoutMs(mcp_call_timeout_ms::Response),
    #[schemars(title = "McpConnectTimeoutMs")]
    McpConnectTimeoutMs(mcp_connect_timeout_ms::Response),
    #[schemars(title = "BackoffMaxElapsedTimeMs")]
    BackoffMaxElapsedTimeMs(backoff_max_elapsed_time_ms::Response),
    #[schemars(title = "UserAgent")]
    UserAgent(user_agent::Response),
    #[schemars(title = "HttpReferer")]
    HttpReferer(http_referer::Response),
    #[schemars(title = "XTitle")]
    XTitle(x_title::Response),
    #[schemars(title = "CommitAuthorName")]
    CommitAuthorName(commit_author_name::Response),
    #[schemars(title = "CommitAuthorEmail")]
    CommitAuthorEmail(commit_author_email::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Get(v) => v.into_mcp(),
            Response::GetRequestSchema(v) => v.into_mcp(),
            Response::GetResponseSchema(v) => v.into_mcp(),
            Response::Address(v) => v.into_mcp(),
            Response::ObjectiveaiAuthorization(v) => v.into_mcp(),
            Response::OpenrouterAuthorization(v) => v.into_mcp(),
            Response::GithubAuthorization(v) => v.into_mcp(),
            Response::McpAuthorization(v) => v.into_mcp(),
            Response::McpCallTimeoutMs(v) => v.into_mcp(),
            Response::McpConnectTimeoutMs(v) => v.into_mcp(),
            Response::BackoffMaxElapsedTimeMs(v) => v.into_mcp(),
            Response::UserAgent(v) => v.into_mcp(),
            Response::HttpReferer(v) => v.into_mcp(),
            Response::XTitle(v) => v.into_mcp(),
            Response::CommitAuthorName(v) => v.into_mcp(),
            Response::CommitAuthorEmail(v) => v.into_mcp(),
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
            Command::ObjectiveaiAuthorization { command } =>
                Ok(Request::ObjectiveaiAuthorization(objectiveai_authorization::Request::try_from(command)?)),
            Command::OpenrouterAuthorization { command } =>
                Ok(Request::OpenrouterAuthorization(openrouter_authorization::Request::try_from(command)?)),
            Command::GithubAuthorization { command } =>
                Ok(Request::GithubAuthorization(github_authorization::Request::try_from(command)?)),
            Command::McpAuthorization { command } =>
                Ok(Request::McpAuthorization(mcp_authorization::Request::try_from(command)?)),
            Command::McpCallTimeoutMs { command } =>
                Ok(Request::McpCallTimeoutMs(mcp_call_timeout_ms::Request::try_from(command)?)),
            Command::McpConnectTimeoutMs { command } =>
                Ok(Request::McpConnectTimeoutMs(mcp_connect_timeout_ms::Request::try_from(command)?)),
            Command::BackoffMaxElapsedTimeMs { command } =>
                Ok(Request::BackoffMaxElapsedTimeMs(backoff_max_elapsed_time_ms::Request::try_from(command)?)),
            Command::UserAgent { command } =>
                Ok(Request::UserAgent(user_agent::Request::try_from(command)?)),
            Command::HttpReferer { command } =>
                Ok(Request::HttpReferer(http_referer::Request::try_from(command)?)),
            Command::XTitle { command } =>
                Ok(Request::XTitle(x_title::Request::try_from(command)?)),
            Command::CommitAuthorName { command } =>
                Ok(Request::CommitAuthorName(commit_author_name::Request::try_from(command)?)),
            Command::CommitAuthorEmail { command } =>
                Ok(Request::CommitAuthorEmail(commit_author_email::Request::try_from(command)?)),
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
            Request::ObjectiveaiAuthorization(inner) => inner.request_base(),
            Request::OpenrouterAuthorization(inner) => inner.request_base(),
            Request::GithubAuthorization(inner) => inner.request_base(),
            Request::McpAuthorization(inner) => inner.request_base(),
            Request::McpCallTimeoutMs(inner) => inner.request_base(),
            Request::McpConnectTimeoutMs(inner) => inner.request_base(),
            Request::BackoffMaxElapsedTimeMs(inner) => inner.request_base(),
            Request::UserAgent(inner) => inner.request_base(),
            Request::HttpReferer(inner) => inner.request_base(),
            Request::XTitle(inner) => inner.request_base(),
            Request::CommitAuthorName(inner) => inner.request_base(),
            Request::CommitAuthorEmail(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Get(inner) => inner.request_base_mut(),
            Request::GetRequestSchema(inner) => inner.request_base_mut(),
            Request::GetResponseSchema(inner) => inner.request_base_mut(),
            Request::Address(inner) => inner.request_base_mut(),
            Request::ObjectiveaiAuthorization(inner) => inner.request_base_mut(),
            Request::OpenrouterAuthorization(inner) => inner.request_base_mut(),
            Request::GithubAuthorization(inner) => inner.request_base_mut(),
            Request::McpAuthorization(inner) => inner.request_base_mut(),
            Request::McpCallTimeoutMs(inner) => inner.request_base_mut(),
            Request::McpConnectTimeoutMs(inner) => inner.request_base_mut(),
            Request::BackoffMaxElapsedTimeMs(inner) => inner.request_base_mut(),
            Request::UserAgent(inner) => inner.request_base_mut(),
            Request::HttpReferer(inner) => inner.request_base_mut(),
            Request::XTitle(inner) => inner.request_base_mut(),
            Request::CommitAuthorName(inner) => inner.request_base_mut(),
            Request::CommitAuthorEmail(inner) => inner.request_base_mut(),
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
            Request::ObjectiveaiAuthorization(req) => {
                let inner = objectiveai_authorization::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::ObjectiveaiAuthorization)))
            }
            Request::OpenrouterAuthorization(req) => {
                let inner = openrouter_authorization::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::OpenrouterAuthorization)))
            }
            Request::GithubAuthorization(req) => {
                let inner = github_authorization::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::GithubAuthorization)))
            }
            Request::McpAuthorization(req) => {
                let inner = mcp_authorization::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::McpAuthorization)))
            }
            Request::McpCallTimeoutMs(req) => {
                let inner = mcp_call_timeout_ms::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::McpCallTimeoutMs)))
            }
            Request::McpConnectTimeoutMs(req) => {
                let inner = mcp_connect_timeout_ms::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::McpConnectTimeoutMs)))
            }
            Request::BackoffMaxElapsedTimeMs(req) => {
                let inner = backoff_max_elapsed_time_ms::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::BackoffMaxElapsedTimeMs)))
            }
            Request::UserAgent(req) => {
                let inner = user_agent::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::UserAgent)))
            }
            Request::HttpReferer(req) => {
                let inner = http_referer::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::HttpReferer)))
            }
            Request::XTitle(req) => {
                let inner = x_title::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::XTitle)))
            }
            Request::CommitAuthorName(req) => {
                let inner = commit_author_name::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::CommitAuthorName)))
            }
            Request::CommitAuthorEmail(req) => {
                let inner = commit_author_email::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::CommitAuthorEmail)))
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
            Request::ObjectiveaiAuthorization(req) => {
                let inner = objectiveai_authorization::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::OpenrouterAuthorization(req) => {
                let inner = openrouter_authorization::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::GithubAuthorization(req) => {
                let inner = github_authorization::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::McpAuthorization(req) => {
                let inner = mcp_authorization::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::McpCallTimeoutMs(req) => {
                let inner = mcp_call_timeout_ms::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::McpConnectTimeoutMs(req) => {
                let inner = mcp_connect_timeout_ms::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::BackoffMaxElapsedTimeMs(req) => {
                let inner = backoff_max_elapsed_time_ms::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::UserAgent(req) => {
                let inner = user_agent::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::HttpReferer(req) => {
                let inner = http_referer::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::XTitle(req) => {
                let inner = x_title::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::CommitAuthorName(req) => {
                let inner = commit_author_name::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::CommitAuthorEmail(req) => {
                let inner = commit_author_email::execute_transform(executor, req, transform, agent_arguments).await?;
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
    Address(address::ListenerExecution),
    ObjectiveaiAuthorization(objectiveai_authorization::ListenerExecution),
    OpenrouterAuthorization(openrouter_authorization::ListenerExecution),
    GithubAuthorization(github_authorization::ListenerExecution),
    McpAuthorization(mcp_authorization::ListenerExecution),
    McpCallTimeoutMs(mcp_call_timeout_ms::ListenerExecution),
    McpConnectTimeoutMs(mcp_connect_timeout_ms::ListenerExecution),
    BackoffMaxElapsedTimeMs(backoff_max_elapsed_time_ms::ListenerExecution),
    UserAgent(user_agent::ListenerExecution),
    HttpReferer(http_referer::ListenerExecution),
    XTitle(x_title::ListenerExecution),
    CommitAuthorName(commit_author_name::ListenerExecution),
    CommitAuthorEmail(commit_author_email::ListenerExecution),
}
