pub mod enqueue;
pub mod get;
pub mod instances;
pub mod list;
pub mod logs;
pub mod message;
pub mod publish;
pub mod queue;
pub mod selector;
pub mod spawn;
pub mod tags;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Park a message in the queue against an agent instance or tag
    /// and return immediately (no delivery race, no spawn).
    Enqueue(enqueue::Command),
    /// Get an agent by remote path.
    Get(get::Command),
    /// Caller-side handles for live spawned agents that didn't earn
    /// their own top-level home: `me`, `list`.
    Instances {
        #[command(subcommand)]
        command: instances::Command,
    },
    /// List remote agents available from a given source.
    List(list::Command),
    /// Persisted log tier — `read` and friends.
    Logs {
        #[command(subcommand)]
        command: logs::Command,
    },
    /// Deliver a message to a running spawned agent (or resume its
    /// most recent completion via continuation if it's dormant).
    Message(message::Command),
    /// Publish an agent to the local filesystem.
    Publish(publish::Command),
    /// Deferred-prompt queue — `delete`, `read`. (Add is gone — use
    /// `agents message` instead.)
    Queue {
        #[command(subcommand)]
        command: queue::Command,
    },
    /// Spawn an agent completion (open a streaming run as a child of
    /// this caller).
    Spawn(spawn::Command),
    /// Client-side agent tags — lookup / apply.
    Tags {
        #[command(subcommand)]
        command: tags::Command,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.Request")]
pub enum Request {
    #[schemars(title = "Enqueue")]
    Enqueue(enqueue::Request),
    #[schemars(title = "EnqueueRequestSchema")]
    EnqueueRequestSchema(enqueue::request_schema::Request),
    #[schemars(title = "EnqueueResponseSchema")]
    EnqueueResponseSchema(enqueue::response_schema::Request),
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
    #[schemars(title = "Logs")]
    Logs(logs::Request),
    #[schemars(title = "Message")]
    Message(message::Request),
    #[schemars(title = "MessageRequestSchema")]
    MessageRequestSchema(message::request_schema::Request),
    #[schemars(title = "MessageResponseSchema")]
    MessageResponseSchema(message::response_schema::Request),
    #[schemars(title = "Publish")]
    Publish(publish::Request),
    #[schemars(title = "PublishRequestSchema")]
    PublishRequestSchema(publish::request_schema::Request),
    #[schemars(title = "PublishResponseSchema")]
    PublishResponseSchema(publish::response_schema::Request),
    #[schemars(title = "Queue")]
    Queue(queue::Request),
    #[schemars(title = "Spawn")]
    Spawn(spawn::Request),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Request),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Request),
    #[schemars(title = "Tags")]
    Tags(tags::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Enqueue")]
    Enqueue(enqueue::Response),
    #[schemars(title = "EnqueueRequestSchema")]
    EnqueueRequestSchema(enqueue::request_schema::Response),
    #[schemars(title = "EnqueueResponseSchema")]
    EnqueueResponseSchema(enqueue::response_schema::Response),
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
    #[schemars(title = "Logs")]
    Logs(logs::ResponseItem),
    #[schemars(title = "Message")]
    Message(message::Response),
    #[schemars(title = "MessageRequestSchema")]
    MessageRequestSchema(message::request_schema::Response),
    #[schemars(title = "MessageResponseSchema")]
    MessageResponseSchema(message::response_schema::Response),
    #[schemars(title = "Publish")]
    Publish(publish::Response),
    #[schemars(title = "PublishRequestSchema")]
    PublishRequestSchema(publish::request_schema::Response),
    #[schemars(title = "PublishResponseSchema")]
    PublishResponseSchema(publish::response_schema::Response),
    #[schemars(title = "Queue")]
    Queue(queue::ResponseItem),
    #[schemars(title = "Spawn")]
    Spawn(spawn::ResponseItem),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Response),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Response),
    #[schemars(title = "Tags")]
    Tags(tags::ResponseItem),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Enqueue(v) => v.into_mcp(),
            ResponseItem::EnqueueRequestSchema(v) => v.into_mcp(),
            ResponseItem::EnqueueResponseSchema(v) => v.into_mcp(),
            ResponseItem::Get(v) => v.into_mcp(),
            ResponseItem::GetRequestSchema(v) => v.into_mcp(),
            ResponseItem::GetResponseSchema(v) => v.into_mcp(),
            ResponseItem::Instances(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
            ResponseItem::Logs(v) => v.into_mcp(),
            ResponseItem::Message(v) => v.into_mcp(),
            ResponseItem::MessageRequestSchema(v) => v.into_mcp(),
            ResponseItem::MessageResponseSchema(v) => v.into_mcp(),
            ResponseItem::Publish(v) => v.into_mcp(),
            ResponseItem::PublishRequestSchema(v) => v.into_mcp(),
            ResponseItem::PublishResponseSchema(v) => v.into_mcp(),
            ResponseItem::Queue(v) => v.into_mcp(),
            ResponseItem::Spawn(v) => v.into_mcp(),
            ResponseItem::SpawnRequestSchema(v) => v.into_mcp(),
            ResponseItem::SpawnResponseSchema(v) => v.into_mcp(),
            ResponseItem::Tags(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Enqueue(cmd) => match cmd.schema {
                None => Ok(Request::Enqueue(enqueue::Request::try_from(cmd.args)?)),
                Some(enqueue::Schema::RequestSchema(args)) =>
                    Ok(Request::EnqueueRequestSchema(enqueue::request_schema::Request::try_from(args)?)),
                Some(enqueue::Schema::ResponseSchema(args)) =>
                    Ok(Request::EnqueueResponseSchema(enqueue::response_schema::Request::try_from(args)?)),
            },
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
            Command::Logs { command } =>
                Ok(Request::Logs(logs::Request::try_from(command)?)),
            Command::Message(cmd) => match cmd.schema {
                None => Ok(Request::Message(message::Request::try_from(cmd.args)?)),
                Some(message::Schema::RequestSchema(args)) =>
                    Ok(Request::MessageRequestSchema(message::request_schema::Request::try_from(args)?)),
                Some(message::Schema::ResponseSchema(args)) =>
                    Ok(Request::MessageResponseSchema(message::response_schema::Request::try_from(args)?)),
            },
            Command::Publish(cmd) => match cmd.schema {
                None => Ok(Request::Publish(publish::Request::try_from(cmd.args)?)),
                Some(publish::Schema::RequestSchema(args)) =>
                    Ok(Request::PublishRequestSchema(publish::request_schema::Request::try_from(args)?)),
                Some(publish::Schema::ResponseSchema(args)) =>
                    Ok(Request::PublishResponseSchema(publish::response_schema::Request::try_from(args)?)),
            },
            Command::Queue { command } =>
                Ok(Request::Queue(queue::Request::try_from(command)?)),
            Command::Spawn(cmd) => match cmd.schema {
                None => Ok(Request::Spawn(spawn::Request::try_from(cmd.args)?)),
                Some(spawn::Schema::RequestSchema(args)) =>
                    Ok(Request::SpawnRequestSchema(spawn::request_schema::Request::try_from(args)?)),
                Some(spawn::Schema::ResponseSchema(args)) =>
                    Ok(Request::SpawnResponseSchema(spawn::response_schema::Request::try_from(args)?)),
            },
            Command::Tags { command } =>
                Ok(Request::Tags(tags::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Enqueue(inner) => inner.into_command(),
            Request::EnqueueRequestSchema(inner) => inner.into_command(),
            Request::EnqueueResponseSchema(inner) => inner.into_command(),
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Instances(inner) => inner.into_command(),
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
            Request::Logs(inner) => inner.into_command(),
            Request::Message(inner) => inner.into_command(),
            Request::MessageRequestSchema(inner) => inner.into_command(),
            Request::MessageResponseSchema(inner) => inner.into_command(),
            Request::Publish(inner) => inner.into_command(),
            Request::PublishRequestSchema(inner) => inner.into_command(),
            Request::PublishResponseSchema(inner) => inner.into_command(),
            Request::Queue(inner) => inner.into_command(),
            Request::Spawn(inner) => inner.into_command(),
            Request::SpawnRequestSchema(inner) => inner.into_command(),
            Request::SpawnResponseSchema(inner) => inner.into_command(),
            Request::Tags(inner) => inner.into_command(),
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
            Request::Enqueue(req) => {
                let value = enqueue::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Enqueue(value),
                )))
            }
            Request::EnqueueRequestSchema(req) => {
                let value = enqueue::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::EnqueueRequestSchema(value),
                )))
            }
            Request::EnqueueResponseSchema(req) => {
                let value = enqueue::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::EnqueueResponseSchema(value),
                )))
            }
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
            Request::Logs(req) => {
                let inner = logs::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Logs)))
            }
            Request::Message(req) => {
                let value = message::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Message(value),
                )))
            }
            Request::MessageRequestSchema(req) => {
                let value = message::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::MessageRequestSchema(value),
                )))
            }
            Request::MessageResponseSchema(req) => {
                let value = message::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::MessageResponseSchema(value),
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
            Request::Queue(req) => {
                let inner = queue::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Queue)))
            }
            Request::Spawn(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = spawn::execute_streaming(executor, req, agent_arguments).await?;
                    Box::pin(inner.map(|r| r.map(ResponseItem::Spawn)))
                } else {
                    let value = spawn::execute(executor, req, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(
                        ResponseItem::Spawn(spawn::ResponseItem::Id(value)),
                    )))
                }
            }
            Request::SpawnRequestSchema(req) => {
                let value = spawn::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SpawnRequestSchema(value),
                )))
            }
            Request::SpawnResponseSchema(req) => {
                let value = spawn::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SpawnResponseSchema(value),
                )))
            }
            Request::Tags(req) => {
                let inner = tags::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Tags)))
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
            Request::Enqueue(req) => {
                let value = enqueue::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::EnqueueRequestSchema(req) => {
                let value =
                    enqueue::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::EnqueueResponseSchema(req) => {
                let value =
                    enqueue::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
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
            Request::Logs(req) => {
                let inner = logs::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Message(req) => {
                let value = message::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::MessageRequestSchema(req) => {
                let value =
                    message::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::MessageResponseSchema(req) => {
                let value =
                    message::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
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
            Request::Queue(req) => {
                let inner = queue::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Spawn(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = spawn::execute_streaming_jq(executor, req, jq, agent_arguments).await?;
                    Box::pin(inner)
                } else {
                    let value = spawn::execute_jq(executor, req, jq, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
                }
            }
            Request::SpawnRequestSchema(req) => {
                let value =
                    spawn::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SpawnResponseSchema(req) => {
                let value =
                    spawn::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Tags(req) => {
                let inner = tags::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
