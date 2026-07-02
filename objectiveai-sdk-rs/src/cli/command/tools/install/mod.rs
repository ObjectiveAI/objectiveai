pub mod filesystem;
pub mod github;

#[derive(clap::Subcommand)]
pub enum Command {
    Filesystem(filesystem::Command),
    Github(github::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.tools.install.Request")]
pub enum Request {
    #[schemars(title = "Filesystem")]
    Filesystem(filesystem::Request),
    #[schemars(title = "FilesystemRequestSchema")]
    FilesystemRequestSchema(filesystem::request_schema::Request),
    #[schemars(title = "FilesystemResponseSchema")]
    FilesystemResponseSchema(filesystem::response_schema::Request),
    #[schemars(title = "Github")]
    Github(github::Request),
    #[schemars(title = "GithubRequestSchema")]
    GithubRequestSchema(github::request_schema::Request),
    #[schemars(title = "GithubResponseSchema")]
    GithubResponseSchema(github::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.install.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Filesystem")]
    Filesystem(filesystem::Response),
    #[schemars(title = "FilesystemRequestSchema")]
    FilesystemRequestSchema(filesystem::request_schema::Response),
    #[schemars(title = "FilesystemResponseSchema")]
    FilesystemResponseSchema(filesystem::response_schema::Response),
    #[schemars(title = "Github")]
    Github(github::Response),
    #[schemars(title = "GithubRequestSchema")]
    GithubRequestSchema(github::request_schema::Response),
    #[schemars(title = "GithubResponseSchema")]
    GithubResponseSchema(github::response_schema::Response),
}

/// Viewer-stream mirror of [`Request`] — the real command requests only
/// (schema-introspection variants are excluded; the viewer streams
/// actual command traffic). Untagged: each variant carries the leaf's
/// `path_type`, so it stays discriminable.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.tools.install.ViewerRequest")]
pub enum ViewerRequest {
    #[schemars(title = "Filesystem")]
    Filesystem(filesystem::ViewerRequest),
    #[schemars(title = "Github")]
    Github(github::ViewerRequest),
}

/// Viewer-stream mirror of [`Response`] — mirrors the base response
/// aggregate: unary children carry their `ViewerResponse`, streaming
/// children their `ViewerResponseItem`. Exempt from json-schema coverage:
/// untagged response aggregate (mirrors the base `Response`, TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.tools.install.ViewerResponseItem")]
pub enum ViewerResponseItem {
    #[schemars(title = "Filesystem")]
    Filesystem(filesystem::ViewerResponse),
    #[schemars(title = "Github")]
    Github(github::ViewerResponse),
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
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Filesystem(inner) => inner.request_base(),
            Request::FilesystemRequestSchema(inner) => inner.request_base(),
            Request::FilesystemResponseSchema(inner) => inner.request_base(),
            Request::Github(inner) => inner.request_base(),
            Request::GithubRequestSchema(inner) => inner.request_base(),
            Request::GithubResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Filesystem(inner) => inner.request_base_mut(),
            Request::FilesystemRequestSchema(inner) => inner.request_base_mut(),
            Request::FilesystemResponseSchema(inner) => inner.request_base_mut(),
            Request::Github(inner) => inner.request_base_mut(),
            Request::GithubRequestSchema(inner) => inner.request_base_mut(),
            Request::GithubResponseSchema(inner) => inner.request_base_mut(),
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
            Request::Filesystem(req) => {
                let value = filesystem::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::FilesystemRequestSchema(req) => {
                let value = filesystem::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::FilesystemResponseSchema(req) => {
                let value = filesystem::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Github(req) => {
                let value = github::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GithubRequestSchema(req) => {
                let value = github::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GithubResponseSchema(req) => {
                let value = github::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
