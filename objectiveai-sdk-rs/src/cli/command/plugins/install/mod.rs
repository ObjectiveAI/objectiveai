pub mod filesystem;
pub mod github;

#[derive(clap::Subcommand)]
pub enum Command {
    Filesystem(filesystem::Command),
    Github(github::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.plugins.install.Request")]
pub enum Request {
    Filesystem(filesystem::Request),
    FilesystemRequestSchema(filesystem::request_schema::Request),
    FilesystemResponseSchema(filesystem::response_schema::Request),
    Github(github::Request),
    GithubRequestSchema(github::request_schema::Request),
    GithubResponseSchema(github::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.install.Response")]
pub enum Response {
    Filesystem(filesystem::Response),
    FilesystemRequestSchema(filesystem::request_schema::Response),
    FilesystemResponseSchema(filesystem::response_schema::Response),
    Github(github::Response),
    GithubRequestSchema(github::request_schema::Response),
    GithubResponseSchema(github::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Filesystem(v) => v.into_mcp(),
            Response::FilesystemRequestSchema(v) => v.into_mcp(),
            Response::FilesystemResponseSchema(v) => v.into_mcp(),
            Response::Github(v) => v.into_mcp(),
            Response::GithubRequestSchema(v) => v.into_mcp(),
            Response::GithubResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Filesystem(cmd) => match cmd.schema {
                None => Ok(Request::Filesystem(filesystem::Request::try_from(cmd.args)?)),
                Some(filesystem::Schema::RequestSchema(args)) =>
                    Ok(Request::FilesystemRequestSchema(filesystem::request_schema::Request::try_from(args)?)),
                Some(filesystem::Schema::ResponseSchema(args)) =>
                    Ok(Request::FilesystemResponseSchema(filesystem::response_schema::Request::try_from(args)?)),
            },
            Command::Github(cmd) => match cmd.schema {
                None => Ok(Request::Github(github::Request::try_from(cmd.args)?)),
                Some(github::Schema::RequestSchema(args)) =>
                    Ok(Request::GithubRequestSchema(github::request_schema::Request::try_from(args)?)),
                Some(github::Schema::ResponseSchema(args)) =>
                    Ok(Request::GithubResponseSchema(github::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Filesystem(inner) => inner.into_command(),
            Request::FilesystemRequestSchema(inner) => inner.into_command(),
            Request::FilesystemResponseSchema(inner) => inner.into_command(),
            Request::Github(inner) => inner.into_command(),
            Request::GithubRequestSchema(inner) => inner.into_command(),
            Request::GithubResponseSchema(inner) => inner.into_command(),
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
            Request::Filesystem(req) => {
                let value = filesystem::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Filesystem(value),
                )))
            }
            Request::FilesystemRequestSchema(req) => {
                let value = filesystem::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::FilesystemRequestSchema(value),
                )))
            }
            Request::FilesystemResponseSchema(req) => {
                let value = filesystem::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::FilesystemResponseSchema(value),
                )))
            }
            Request::Github(req) => {
                let value = github::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Github(value),
                )))
            }
            Request::GithubRequestSchema(req) => {
                let value = github::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GithubRequestSchema(value),
                )))
            }
            Request::GithubResponseSchema(req) => {
                let value = github::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GithubResponseSchema(value),
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
            Request::Filesystem(req) => {
                let value = filesystem::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::FilesystemRequestSchema(req) => {
                let value = filesystem::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::FilesystemResponseSchema(req) => {
                let value = filesystem::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Github(req) => {
                let value = github::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GithubRequestSchema(req) => {
                let value = github::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GithubResponseSchema(req) => {
                let value = github::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
