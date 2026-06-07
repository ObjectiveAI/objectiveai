pub mod get;
pub mod instances;
pub mod list;
pub mod message_queue;
pub mod publish;
pub mod tags;
pub mod tasks;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Get an agent by remote path or favorite name.
    Get(get::Command),
    /// Caller-side handles for live spawned agents:
    /// `spawn`, `message`, `read`, `me`, `list`.
    Instances {
        #[command(subcommand)]
        command: instances::Command,
    },
    /// List remote agents available from a given source.
    List(list::Command),
    /// Publish an agent to the local filesystem.
    Publish(publish::Command),
    /// Deferred-prompt queue — `add` (more leaves to follow).
    MessageQueue {
        #[command(subcommand)]
        command: message_queue::Command,
    },
    /// Client-side agent tags — lookup / add.
    Tags {
        #[command(subcommand)]
        command: tags::Command,
    },
    /// Task store + (eventual) runner. Today: `tasks schedule`
    /// registers a command + interval in `tasks.sqlite`.
    Tasks {
        #[command(subcommand)]
        command: tasks::Command,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.Request")]
pub enum Request {
    #[schemars(title = "Get")]
    Get(get::Request),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Request),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Request),
    #[schemars(title = "Instances")]
    Instances(instances::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
    #[schemars(title = "Publish")]
    Publish(publish::Request),
    #[schemars(title = "PublishRequestSchema")]
    PublishRequestSchema(publish::request_schema::Request),
    #[schemars(title = "PublishResponseSchema")]
    PublishResponseSchema(publish::response_schema::Request),
    #[schemars(title = "MessageQueue")]
    MessageQueue(message_queue::Request),
    #[schemars(title = "Tags")]
    Tags(tags::Request),
    #[schemars(title = "Tasks")]
    Tasks(tasks::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Get")]
    Get(get::Response),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Response),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Response),
    #[schemars(title = "Instances")]
    Instances(instances::ResponseItem),
    #[schemars(title = "List")]
    List(list::ResponseItem),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
    #[schemars(title = "Publish")]
    Publish(publish::Response),
    #[schemars(title = "PublishRequestSchema")]
    PublishRequestSchema(publish::request_schema::Response),
    #[schemars(title = "PublishResponseSchema")]
    PublishResponseSchema(publish::response_schema::Response),
    #[schemars(title = "MessageQueue")]
    MessageQueue(message_queue::ResponseItem),
    #[schemars(title = "Tags")]
    Tags(tags::ResponseItem),
    #[schemars(title = "Tasks")]
    Tasks(tasks::ResponseItem),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Get(v) => v.into_mcp(),
            ResponseItem::GetRequestSchema(v) => v.into_mcp(),
            ResponseItem::GetResponseSchema(v) => v.into_mcp(),
            ResponseItem::Instances(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
            ResponseItem::Publish(v) => v.into_mcp(),
            ResponseItem::PublishRequestSchema(v) => v.into_mcp(),
            ResponseItem::PublishResponseSchema(v) => v.into_mcp(),
            ResponseItem::MessageQueue(v) => v.into_mcp(),
            ResponseItem::Tags(v) => v.into_mcp(),
            ResponseItem::Tasks(v) => v.into_mcp(),
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
            Command::Instances { command } =>
                Ok(Request::Instances(instances::Request::try_from(command)?)),
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) =>
                    Ok(Request::ListRequestSchema(list::request_schema::Request::try_from(args)?)),
                Some(list::Schema::ResponseSchema(args)) =>
                    Ok(Request::ListResponseSchema(list::response_schema::Request::try_from(args)?)),
            },
            Command::Publish(cmd) => match cmd.schema {
                None => Ok(Request::Publish(publish::Request::try_from(cmd.args)?)),
                Some(publish::Schema::RequestSchema(args)) =>
                    Ok(Request::PublishRequestSchema(publish::request_schema::Request::try_from(args)?)),
                Some(publish::Schema::ResponseSchema(args)) =>
                    Ok(Request::PublishResponseSchema(publish::response_schema::Request::try_from(args)?)),
            },
            Command::MessageQueue { command } =>
                Ok(Request::MessageQueue(message_queue::Request::try_from(command)?)),
            Command::Tags { command } =>
                Ok(Request::Tags(tags::Request::try_from(command)?)),
            Command::Tasks { command } =>
                Ok(Request::Tasks(tasks::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Instances(inner) => inner.into_command(),
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
            Request::Publish(inner) => inner.into_command(),
            Request::PublishRequestSchema(inner) => inner.into_command(),
            Request::PublishResponseSchema(inner) => inner.into_command(),
            Request::MessageQueue(inner) => inner.into_command(),
            Request::Tags(inner) => inner.into_command(),
            Request::Tasks(inner) => inner.into_command(),
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
            Request::Instances(req) => {
                let inner = instances::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Instances)))
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
            Request::Publish(req) => {
                let value = publish::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Publish(value),
                )))
            }
            Request::PublishRequestSchema(req) => {
                let value = publish::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::PublishRequestSchema(value),
                )))
            }
            Request::PublishResponseSchema(req) => {
                let value = publish::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::PublishResponseSchema(value),
                )))
            }
            Request::MessageQueue(req) => {
                let inner = message_queue::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::MessageQueue)))
            }
            Request::Tags(req) => {
                let inner = tags::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Tags)))
            }
            Request::Tasks(req) => {
                let inner = tasks::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Tasks)))
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
            Request::Get(req) => {
                let value = get::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Instances(req) => {
                let inner = instances::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::List(req) => {
                let inner = list::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::ListRequestSchema(req) => {
                let value = list::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ListResponseSchema(req) => {
                let value = list::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Publish(req) => {
                let value = publish::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::PublishRequestSchema(req) => {
                let value = publish::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::PublishResponseSchema(req) => {
                let value = publish::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::MessageQueue(req) => {
                let inner = message_queue::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Tags(req) => {
                let inner = tags::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Tasks(req) => {
                let inner = tasks::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
