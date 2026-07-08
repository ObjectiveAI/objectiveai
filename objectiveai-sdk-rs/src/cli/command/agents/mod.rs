pub mod enqueue;
pub mod get;
pub mod instances;
pub mod laboratories;
pub mod list;
pub mod logs;
pub mod mcp;
pub mod message;
pub mod publish;
pub mod queue;
pub mod selector;
pub mod spawn;
pub mod tags;
pub mod wait;

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
    /// Attach/detach/list laboratory ids on an agent target.
    Laboratories {
        #[command(subcommand)]
        command: laboratories::Command,
    },
    /// List remote agents available from a given source.
    List(list::Command),
    /// Persisted log tier — `open`, `list`, `subscribe`.
    Logs {
        #[command(subcommand)]
        command: logs::Command,
    },
    /// Query a live agent's aggregated MCP surface — `resources`,
    /// `tools` — over its per-`response_id` listener socket.
    Mcp {
        #[command(subcommand)]
        command: mcp::Command,
    },
    /// Deliver a message to a running spawned agent (or resume its
    /// most recent completion via continuation if it's dormant).
    Message(message::Command),
    /// Publish an agent to the local filesystem.
    Publish(publish::Command),
    /// Deferred-prompt queue — `open`, `list`, `delete`, `deliver`.
    /// (Add is gone — use `agents message` instead.)
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
    /// Block until an agent (instance or tag) is done — its lock
    /// chain fully released — or the timeout elapses.
    Wait(wait::Command),
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
    #[schemars(title = "Laboratories")]
    Laboratories(laboratories::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
    #[schemars(title = "Logs")]
    Logs(logs::Request),
    #[schemars(title = "Mcp")]
    Mcp(mcp::Request),
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
    #[schemars(title = "Wait")]
    Wait(wait::Request),
    #[schemars(title = "WaitRequestSchema")]
    WaitRequestSchema(wait::request_schema::Request),
    #[schemars(title = "WaitResponseSchema")]
    WaitResponseSchema(wait::response_schema::Request),
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
    #[schemars(title = "Laboratories")]
    Laboratories(laboratories::ResponseItem),
    #[schemars(title = "List")]
    List(list::ResponseItem),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
    #[schemars(title = "Logs")]
    Logs(logs::ResponseItem),
    #[schemars(title = "Mcp")]
    Mcp(mcp::ResponseItem),
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
    #[schemars(title = "Wait")]
    Wait(wait::Response),
    #[schemars(title = "WaitRequestSchema")]
    WaitRequestSchema(wait::request_schema::Response),
    #[schemars(title = "WaitResponseSchema")]
    WaitResponseSchema(wait::response_schema::Response),
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
            ResponseItem::Laboratories(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
            ResponseItem::Logs(v) => v.into_mcp(),
            ResponseItem::Mcp(v) => v.into_mcp(),
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
            ResponseItem::Wait(v) => v.into_mcp(),
            ResponseItem::WaitRequestSchema(v) => v.into_mcp(),
            ResponseItem::WaitResponseSchema(v) => v.into_mcp(),
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
            Command::Laboratories { command } =>
                Ok(Request::Laboratories(laboratories::Request::try_from(command)?)),
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) =>
                    Ok(Request::ListRequestSchema(list::request_schema::Request::try_from(args)?)),
                Some(list::Schema::ResponseSchema(args)) =>
                    Ok(Request::ListResponseSchema(list::response_schema::Request::try_from(args)?)),
            },
            Command::Logs { command } =>
                Ok(Request::Logs(logs::Request::try_from(command)?)),
            Command::Mcp { command } =>
                Ok(Request::Mcp(mcp::Request::try_from(command)?)),
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
            Command::Wait(cmd) => match cmd.schema {
                None => Ok(Request::Wait(wait::Request::try_from(cmd.args)?)),
                Some(wait::Schema::RequestSchema(args)) =>
                    Ok(Request::WaitRequestSchema(wait::request_schema::Request::try_from(args)?)),
                Some(wait::Schema::ResponseSchema(args)) =>
                    Ok(Request::WaitResponseSchema(wait::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Enqueue(inner) => inner.request_base(),
            Request::EnqueueRequestSchema(inner) => inner.request_base(),
            Request::EnqueueResponseSchema(inner) => inner.request_base(),
            Request::Get(inner) => inner.request_base(),
            Request::GetRequestSchema(inner) => inner.request_base(),
            Request::GetResponseSchema(inner) => inner.request_base(),
            Request::Instances(inner) => inner.request_base(),
            Request::Laboratories(inner) => inner.request_base(),
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
            Request::Logs(inner) => inner.request_base(),
            Request::Mcp(inner) => inner.request_base(),
            Request::Message(inner) => inner.request_base(),
            Request::MessageRequestSchema(inner) => inner.request_base(),
            Request::MessageResponseSchema(inner) => inner.request_base(),
            Request::Publish(inner) => inner.request_base(),
            Request::PublishRequestSchema(inner) => inner.request_base(),
            Request::PublishResponseSchema(inner) => inner.request_base(),
            Request::Queue(inner) => inner.request_base(),
            Request::Spawn(inner) => inner.request_base(),
            Request::SpawnRequestSchema(inner) => inner.request_base(),
            Request::SpawnResponseSchema(inner) => inner.request_base(),
            Request::Tags(inner) => inner.request_base(),
            Request::Wait(inner) => inner.request_base(),
            Request::WaitRequestSchema(inner) => inner.request_base(),
            Request::WaitResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Enqueue(inner) => inner.request_base_mut(),
            Request::EnqueueRequestSchema(inner) => inner.request_base_mut(),
            Request::EnqueueResponseSchema(inner) => inner.request_base_mut(),
            Request::Get(inner) => inner.request_base_mut(),
            Request::GetRequestSchema(inner) => inner.request_base_mut(),
            Request::GetResponseSchema(inner) => inner.request_base_mut(),
            Request::Instances(inner) => inner.request_base_mut(),
            Request::Laboratories(inner) => inner.request_base_mut(),
            Request::List(inner) => inner.request_base_mut(),
            Request::ListRequestSchema(inner) => inner.request_base_mut(),
            Request::ListResponseSchema(inner) => inner.request_base_mut(),
            Request::Logs(inner) => inner.request_base_mut(),
            Request::Mcp(inner) => inner.request_base_mut(),
            Request::Message(inner) => inner.request_base_mut(),
            Request::MessageRequestSchema(inner) => inner.request_base_mut(),
            Request::MessageResponseSchema(inner) => inner.request_base_mut(),
            Request::Publish(inner) => inner.request_base_mut(),
            Request::PublishRequestSchema(inner) => inner.request_base_mut(),
            Request::PublishResponseSchema(inner) => inner.request_base_mut(),
            Request::Queue(inner) => inner.request_base_mut(),
            Request::Spawn(inner) => inner.request_base_mut(),
            Request::SpawnRequestSchema(inner) => inner.request_base_mut(),
            Request::SpawnResponseSchema(inner) => inner.request_base_mut(),
            Request::Tags(inner) => inner.request_base_mut(),
            Request::Wait(inner) => inner.request_base_mut(),
            Request::WaitRequestSchema(inner) => inner.request_base_mut(),
            Request::WaitResponseSchema(inner) => inner.request_base_mut(),
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
            Request::Laboratories(req) => {
                let inner = laboratories::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Laboratories)))
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
            Request::Mcp(req) => {
                let inner = mcp::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
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
            Request::Wait(req) => {
                let value = wait::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Wait(value),
                )))
            }
            Request::WaitRequestSchema(req) => {
                let value = wait::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::WaitRequestSchema(value),
                )))
            }
            Request::WaitResponseSchema(req) => {
                let value = wait::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::WaitResponseSchema(value),
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
            Request::Enqueue(req) => {
                let value = enqueue::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::EnqueueRequestSchema(req) => {
                let value =
                    enqueue::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::EnqueueResponseSchema(req) => {
                let value =
                    enqueue::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
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
            Request::Instances(req) => {
                let inner = instances::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Laboratories(req) => {
                let inner = laboratories::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
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
            Request::Logs(req) => {
                let inner = logs::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Mcp(req) => {
                let inner = mcp::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Message(req) => {
                let value = message::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::MessageRequestSchema(req) => {
                let value =
                    message::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::MessageResponseSchema(req) => {
                let value =
                    message::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Publish(req) => {
                let value = publish::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::PublishRequestSchema(req) => {
                let value = publish::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::PublishResponseSchema(req) => {
                let value = publish::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Queue(req) => {
                let inner = queue::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Spawn(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = spawn::execute_streaming_transform(executor, req, transform, agent_arguments).await?;
                    Box::pin(inner)
                } else {
                    let value = spawn::execute_transform(executor, req, transform, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
                }
            }
            Request::SpawnRequestSchema(req) => {
                let value =
                    spawn::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SpawnResponseSchema(req) => {
                let value =
                    spawn::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Tags(req) => {
                let inner = tags::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Wait(req) => {
                let value = wait::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::WaitRequestSchema(req) => {
                let value =
                    wait::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::WaitResponseSchema(req) => {
                let value =
                    wait::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Enqueue(enqueue::ListenerExecution),
    EnqueueRequestSchema(enqueue::request_schema::ListenerExecution),
    EnqueueResponseSchema(enqueue::response_schema::ListenerExecution),
    Get(get::ListenerExecution),
    GetRequestSchema(get::request_schema::ListenerExecution),
    GetResponseSchema(get::response_schema::ListenerExecution),
    Instances(instances::ListenerExecution),
    Laboratories(laboratories::ListenerExecution),
    List(list::ListenerExecution),
    ListRequestSchema(list::request_schema::ListenerExecution),
    ListResponseSchema(list::response_schema::ListenerExecution),
    Logs(logs::ListenerExecution),
    Mcp(mcp::ListenerExecution),
    Message(message::ListenerExecution),
    MessageRequestSchema(message::request_schema::ListenerExecution),
    MessageResponseSchema(message::response_schema::ListenerExecution),
    Publish(publish::ListenerExecution),
    PublishRequestSchema(publish::request_schema::ListenerExecution),
    PublishResponseSchema(publish::response_schema::ListenerExecution),
    Queue(queue::ListenerExecution),
    Spawn(spawn::ListenerExecutionVariant),
    SpawnRequestSchema(spawn::request_schema::ListenerExecution),
    SpawnResponseSchema(spawn::response_schema::ListenerExecution),
    Tags(tags::ListenerExecution),
    Wait(wait::ListenerExecution),
    WaitRequestSchema(wait::request_schema::ListenerExecution),
    WaitResponseSchema(wait::response_schema::ListenerExecution),
}
