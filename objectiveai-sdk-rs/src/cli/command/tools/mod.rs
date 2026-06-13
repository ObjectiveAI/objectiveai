pub mod get;
pub mod install;
pub mod list;
pub mod run;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Install(install::Command),
    List(list::Command),
    Run(run::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.tools.Request")]
pub enum Request {
    #[schemars(title = "Get")]
    Get(get::Request),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Request),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Request),
    #[schemars(title = "Install")]
    Install(install::Request),
    #[schemars(title = "InstallRequestSchema")]
    InstallRequestSchema(install::request_schema::Request),
    #[schemars(title = "InstallResponseSchema")]
    InstallResponseSchema(install::response_schema::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
    #[schemars(title = "Run")]
    Run(run::Request),
    #[schemars(title = "RunRequestSchema")]
    RunRequestSchema(run::request_schema::Request),
    #[schemars(title = "RunResponseSchema")]
    RunResponseSchema(run::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Get")]
    Get(get::Response),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Response),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Response),
    #[schemars(title = "Install")]
    Install(install::Response),
    #[schemars(title = "InstallRequestSchema")]
    InstallRequestSchema(install::request_schema::Response),
    #[schemars(title = "InstallResponseSchema")]
    InstallResponseSchema(install::response_schema::Response),
    #[schemars(title = "List")]
    List(list::ResponseItem),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
    #[schemars(title = "Run")]
    Run(run::ResponseItem),
    #[schemars(title = "RunRequestSchema")]
    RunRequestSchema(run::request_schema::Response),
    #[schemars(title = "RunResponseSchema")]
    RunResponseSchema(run::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Get(v) => v.into_mcp(),
            ResponseItem::GetRequestSchema(v) => v.into_mcp(),
            ResponseItem::GetResponseSchema(v) => v.into_mcp(),
            ResponseItem::Install(v) => v.into_mcp(),
            ResponseItem::InstallRequestSchema(v) => v.into_mcp(),
            ResponseItem::InstallResponseSchema(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
            ResponseItem::Run(v) => v.into_mcp(),
            ResponseItem::RunRequestSchema(v) => v.into_mcp(),
            ResponseItem::RunResponseSchema(v) => v.into_mcp(),
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
            Command::Install(cmd) => match cmd.schema {
                None => Ok(Request::Install(install::Request::try_from(cmd.args)?)),
                Some(install::Schema::RequestSchema(args)) =>
                    Ok(Request::InstallRequestSchema(install::request_schema::Request::try_from(args)?)),
                Some(install::Schema::ResponseSchema(args)) =>
                    Ok(Request::InstallResponseSchema(install::response_schema::Request::try_from(args)?)),
            },
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) =>
                    Ok(Request::ListRequestSchema(list::request_schema::Request::try_from(args)?)),
                Some(list::Schema::ResponseSchema(args)) =>
                    Ok(Request::ListResponseSchema(list::response_schema::Request::try_from(args)?)),
            },
            Command::Run(cmd) => match cmd.schema {
                None => Ok(Request::Run(run::Request::try_from(cmd.args)?)),
                Some(run::Schema::RequestSchema(args)) =>
                    Ok(Request::RunRequestSchema(run::request_schema::Request::try_from(args)?)),
                Some(run::Schema::ResponseSchema(args)) =>
                    Ok(Request::RunResponseSchema(run::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Install(inner) => inner.into_command(),
            Request::InstallRequestSchema(inner) => inner.into_command(),
            Request::InstallResponseSchema(inner) => inner.into_command(),
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
            Request::Run(inner) => inner.into_command(),
            Request::RunRequestSchema(inner) => inner.into_command(),
            Request::RunResponseSchema(inner) => inner.into_command(),
        }
    }

    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Get(inner) => inner.request_base(),
            Request::GetRequestSchema(inner) => inner.request_base(),
            Request::GetResponseSchema(inner) => inner.request_base(),
            Request::Install(inner) => inner.request_base(),
            Request::InstallRequestSchema(inner) => inner.request_base(),
            Request::InstallResponseSchema(inner) => inner.request_base(),
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
            Request::Run(inner) => inner.request_base(),
            Request::RunRequestSchema(inner) => inner.request_base(),
            Request::RunResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Get(inner) => inner.request_base_mut(),
            Request::GetRequestSchema(inner) => inner.request_base_mut(),
            Request::GetResponseSchema(inner) => inner.request_base_mut(),
            Request::Install(inner) => inner.request_base_mut(),
            Request::InstallRequestSchema(inner) => inner.request_base_mut(),
            Request::InstallResponseSchema(inner) => inner.request_base_mut(),
            Request::List(inner) => inner.request_base_mut(),
            Request::ListRequestSchema(inner) => inner.request_base_mut(),
            Request::ListResponseSchema(inner) => inner.request_base_mut(),
            Request::Run(inner) => inner.request_base_mut(),
            Request::RunRequestSchema(inner) => inner.request_base_mut(),
            Request::RunResponseSchema(inner) => inner.request_base_mut(),
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
            Request::Get(req) => {
                let value = get::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Get(value),
                )))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::GetRequestSchema(value),
                )))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::GetResponseSchema(value),
                )))
            }
            Request::Install(req) => {
                let value = install::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Install(value),
                )))
            }
            Request::InstallRequestSchema(req) => {
                let value = install::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::InstallRequestSchema(value),
                )))
            }
            Request::InstallResponseSchema(req) => {
                let value = install::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::InstallResponseSchema(value),
                )))
            }
            Request::List(req) => {
                let inner = list::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::List)))
            }
            Request::ListRequestSchema(req) => {
                let value = list::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ListRequestSchema(value),
                )))
            }
            Request::ListResponseSchema(req) => {
                let value = list::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ListResponseSchema(value),
                )))
            }
            Request::Run(req) => {
                let inner = run::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Run)))
            }
            Request::RunRequestSchema(req) => {
                let value = run::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::RunRequestSchema(value),
                )))
            }
            Request::RunResponseSchema(req) => {
                let value = run::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::RunResponseSchema(value),
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
            Request::Install(req) => {
                let value = install::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::InstallRequestSchema(req) => {
                let value = install::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::InstallResponseSchema(req) => {
                let value = install::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::List(req) => {
                let inner = list::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::ListRequestSchema(req) => {
                let value = list::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ListResponseSchema(req) => {
                let value = list::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Run(req) => {
                let inner = run::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::RunRequestSchema(req) => {
                let value = run::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::RunResponseSchema(req) => {
                let value = run::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
