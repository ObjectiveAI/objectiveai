pub mod all;
pub mod id;
pub mod pending;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Read all items from each agent_instance_hierarchy.
    All(all::Command),
    /// Read a single item by its row id.
    Id(id::Command),
    /// Read pending items only.
    Pending(pending::Command),
    /// Subscribe to live updates for the given agents.
    Subscribe(subscribe::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.read.Request")]
pub enum Request {
    #[schemars(title = "All")]
    All(all::Request),
    #[schemars(title = "AllRequestSchema")]
    AllRequestSchema(all::request_schema::Request),
    #[schemars(title = "AllResponseSchema")]
    AllResponseSchema(all::response_schema::Request),
    #[schemars(title = "Id")]
    Id(id::Request),
    #[schemars(title = "IdRequestSchema")]
    IdRequestSchema(id::request_schema::Request),
    #[schemars(title = "IdResponseSchema")]
    IdResponseSchema(id::response_schema::Request),
    #[schemars(title = "Pending")]
    Pending(pending::Request),
    #[schemars(title = "PendingRequestSchema")]
    PendingRequestSchema(pending::request_schema::Request),
    #[schemars(title = "PendingResponseSchema")]
    PendingResponseSchema(pending::response_schema::Request),
    #[schemars(title = "Subscribe")]
    Subscribe(subscribe::Request),
    #[schemars(title = "SubscribeRequestSchema")]
    SubscribeRequestSchema(subscribe::request_schema::Request),
    #[schemars(title = "SubscribeResponseSchema")]
    SubscribeResponseSchema(subscribe::response_schema::Request),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.read.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "All")]
    All(all::ResponseItem),
    #[schemars(title = "AllRequestSchema")]
    AllRequestSchema(all::request_schema::Response),
    #[schemars(title = "AllResponseSchema")]
    AllResponseSchema(all::response_schema::Response),
    #[schemars(title = "Id")]
    Id(id::Response),
    #[schemars(title = "IdRequestSchema")]
    IdRequestSchema(id::request_schema::Response),
    #[schemars(title = "IdResponseSchema")]
    IdResponseSchema(id::response_schema::Response),
    #[schemars(title = "Pending")]
    Pending(pending::ResponseItem),
    #[schemars(title = "PendingRequestSchema")]
    PendingRequestSchema(pending::request_schema::Response),
    #[schemars(title = "PendingResponseSchema")]
    PendingResponseSchema(pending::response_schema::Response),
    #[schemars(title = "Subscribe")]
    Subscribe(subscribe::ResponseItem),
    #[schemars(title = "SubscribeRequestSchema")]
    SubscribeRequestSchema(subscribe::request_schema::Response),
    #[schemars(title = "SubscribeResponseSchema")]
    SubscribeResponseSchema(subscribe::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::All(v) => v.into_mcp(),
            ResponseItem::AllRequestSchema(v) => v.into_mcp(),
            ResponseItem::AllResponseSchema(v) => v.into_mcp(),
            ResponseItem::Id(v) => v.into_mcp(),
            ResponseItem::IdRequestSchema(v) => v.into_mcp(),
            ResponseItem::IdResponseSchema(v) => v.into_mcp(),
            ResponseItem::Pending(v) => v.into_mcp(),
            ResponseItem::PendingRequestSchema(v) => v.into_mcp(),
            ResponseItem::PendingResponseSchema(v) => v.into_mcp(),
            ResponseItem::Subscribe(v) => v.into_mcp(),
            ResponseItem::SubscribeRequestSchema(v) => v.into_mcp(),
            ResponseItem::SubscribeResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::All(cmd) => match cmd.schema {
                None => Ok(Request::All(all::Request::try_from(cmd.args)?)),
                Some(all::Schema::RequestSchema(args)) =>
                    Ok(Request::AllRequestSchema(all::request_schema::Request::try_from(args)?)),
                Some(all::Schema::ResponseSchema(args)) =>
                    Ok(Request::AllResponseSchema(all::response_schema::Request::try_from(args)?)),
            },
            Command::Id(cmd) => match cmd.schema {
                None => Ok(Request::Id(id::Request::try_from(cmd.args)?)),
                Some(id::Schema::RequestSchema(args)) =>
                    Ok(Request::IdRequestSchema(id::request_schema::Request::try_from(args)?)),
                Some(id::Schema::ResponseSchema(args)) =>
                    Ok(Request::IdResponseSchema(id::response_schema::Request::try_from(args)?)),
            },
            Command::Pending(cmd) => match cmd.schema {
                None => Ok(Request::Pending(pending::Request::try_from(cmd.args)?)),
                Some(pending::Schema::RequestSchema(args)) =>
                    Ok(Request::PendingRequestSchema(pending::request_schema::Request::try_from(args)?)),
                Some(pending::Schema::ResponseSchema(args)) =>
                    Ok(Request::PendingResponseSchema(pending::response_schema::Request::try_from(args)?)),
            },
            Command::Subscribe(cmd) => match cmd.schema {
                None => Ok(Request::Subscribe(subscribe::Request::try_from(cmd.args)?)),
                Some(subscribe::Schema::RequestSchema(args)) =>
                    Ok(Request::SubscribeRequestSchema(subscribe::request_schema::Request::try_from(args)?)),
                Some(subscribe::Schema::ResponseSchema(args)) =>
                    Ok(Request::SubscribeResponseSchema(subscribe::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::All(inner) => inner.into_command(),
            Request::AllRequestSchema(inner) => inner.into_command(),
            Request::AllResponseSchema(inner) => inner.into_command(),
            Request::Id(inner) => inner.into_command(),
            Request::IdRequestSchema(inner) => inner.into_command(),
            Request::IdResponseSchema(inner) => inner.into_command(),
            Request::Pending(inner) => inner.into_command(),
            Request::PendingRequestSchema(inner) => inner.into_command(),
            Request::PendingResponseSchema(inner) => inner.into_command(),
            Request::Subscribe(inner) => inner.into_command(),
            Request::SubscribeRequestSchema(inner) => inner.into_command(),
            Request::SubscribeResponseSchema(inner) => inner.into_command(),
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
            Request::All(req) => {
                let inner = all::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::All)))
            }
            Request::AllRequestSchema(req) => {
                let value = all::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::AllRequestSchema(value),
                )))
            }
            Request::AllResponseSchema(req) => {
                let value = all::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::AllResponseSchema(value),
                )))
            }
            Request::Id(req) => {
                let value = id::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Id(value),
                )))
            }
            Request::IdRequestSchema(req) => {
                let value = id::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::IdRequestSchema(value),
                )))
            }
            Request::IdResponseSchema(req) => {
                let value = id::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::IdResponseSchema(value),
                )))
            }
            Request::Pending(req) => {
                let inner = pending::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Pending)))
            }
            Request::PendingRequestSchema(req) => {
                let value = pending::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::PendingRequestSchema(value),
                )))
            }
            Request::PendingResponseSchema(req) => {
                let value = pending::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::PendingResponseSchema(value),
                )))
            }
            Request::Subscribe(req) => {
                let inner = subscribe::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Subscribe)))
            }
            Request::SubscribeRequestSchema(req) => {
                let value = subscribe::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SubscribeRequestSchema(value),
                )))
            }
            Request::SubscribeResponseSchema(req) => {
                let value = subscribe::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SubscribeResponseSchema(value),
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
            Request::All(req) => {
                let inner = all::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::AllRequestSchema(req) => {
                let value = all::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::AllResponseSchema(req) => {
                let value = all::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Id(req) => {
                let value = id::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::IdRequestSchema(req) => {
                let value = id::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::IdResponseSchema(req) => {
                let value = id::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Pending(req) => {
                let inner = pending::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::PendingRequestSchema(req) => {
                let value = pending::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::PendingResponseSchema(req) => {
                let value = pending::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Subscribe(req) => {
                let inner = subscribe::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::SubscribeRequestSchema(req) => {
                let value = subscribe::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SubscribeResponseSchema(req) => {
                let value = subscribe::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
