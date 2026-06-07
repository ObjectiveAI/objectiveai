//! `agents instances` — caller-side handles for live spawned agents.
//!
//! Five leaves today:
//! - `spawn` — open a streaming run as a child of this caller.
//! - `message` — deliver a message to a running spawned agent.
//! - `read` — read queue items (with `id`, `pending`, `subscribe`,
//!   `all` sub-leaves).
//! - `me` — return the configured self agent id.
//! - `list` — list direct children of the calling agent.

use crate::cli::command::CommandRequest;

pub mod list;
pub mod me;
pub mod message;
pub mod read;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Spawn an agent completion (open a streaming run as a child of this caller).
    Spawn(spawn::Command),
    /// Deliver a message to a running spawned agent (or resume its most
    /// recent completion via continuation if it's dormant).
    Message(message::Command),
    /// Read queue items.
    Read {
        #[command(subcommand)]
        command: read::Command,
    },
    /// Return the configured self agent id.
    Me(me::Command),
    /// List direct children of the calling agent.
    List(list::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.instances.Request")]
pub enum Request {
    #[schemars(title = "Spawn")]
    Spawn(spawn::Request),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Request),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Request),
    #[schemars(title = "Message")]
    Message(message::Request),
    #[schemars(title = "MessageRequestSchema")]
    MessageRequestSchema(message::request_schema::Request),
    #[schemars(title = "MessageResponseSchema")]
    MessageResponseSchema(message::response_schema::Request),
    #[schemars(title = "Read")]
    Read(read::Request),
    #[schemars(title = "Me")]
    Me(me::Request),
    #[schemars(title = "MeRequestSchema")]
    MeRequestSchema(me::request_schema::Request),
    #[schemars(title = "MeResponseSchema")]
    MeResponseSchema(me::response_schema::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Spawn")]
    Spawn(spawn::ResponseItem),
    #[schemars(title = "SpawnRequestSchema")]
    SpawnRequestSchema(spawn::request_schema::Response),
    #[schemars(title = "SpawnResponseSchema")]
    SpawnResponseSchema(spawn::response_schema::Response),
    #[schemars(title = "Message")]
    Message(message::ResponseItem),
    #[schemars(title = "MessageRequestSchema")]
    MessageRequestSchema(message::request_schema::Response),
    #[schemars(title = "MessageResponseSchema")]
    MessageResponseSchema(message::response_schema::Response),
    #[schemars(title = "Read")]
    Read(read::ResponseItem),
    #[schemars(title = "Me")]
    Me(me::Response),
    #[schemars(title = "MeRequestSchema")]
    MeRequestSchema(me::request_schema::Response),
    #[schemars(title = "MeResponseSchema")]
    MeResponseSchema(me::response_schema::Response),
    #[schemars(title = "List")]
    List(list::ResponseItem),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Spawn(v) => v.into_mcp(),
            ResponseItem::SpawnRequestSchema(v) => v.into_mcp(),
            ResponseItem::SpawnResponseSchema(v) => v.into_mcp(),
            ResponseItem::Message(v) => v.into_mcp(),
            ResponseItem::MessageRequestSchema(v) => v.into_mcp(),
            ResponseItem::MessageResponseSchema(v) => v.into_mcp(),
            ResponseItem::Read(v) => v.into_mcp(),
            ResponseItem::Me(v) => v.into_mcp(),
            ResponseItem::MeRequestSchema(v) => v.into_mcp(),
            ResponseItem::MeResponseSchema(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Spawn(cmd) => match cmd.schema {
                None => Ok(Request::Spawn(spawn::Request::try_from(cmd.args)?)),
                Some(spawn::Schema::RequestSchema(args)) =>
                    Ok(Request::SpawnRequestSchema(spawn::request_schema::Request::try_from(args)?)),
                Some(spawn::Schema::ResponseSchema(args)) =>
                    Ok(Request::SpawnResponseSchema(spawn::response_schema::Request::try_from(args)?)),
            },
            Command::Message(cmd) => match cmd.schema {
                None => Ok(Request::Message(message::Request::try_from(cmd.args)?)),
                Some(message::Schema::RequestSchema(args)) =>
                    Ok(Request::MessageRequestSchema(message::request_schema::Request::try_from(args)?)),
                Some(message::Schema::ResponseSchema(args)) =>
                    Ok(Request::MessageResponseSchema(message::response_schema::Request::try_from(args)?)),
            },
            Command::Read { command } =>
                Ok(Request::Read(read::Request::try_from(command)?)),
            Command::Me(cmd) => match cmd.schema {
                None => Ok(Request::Me(me::Request::try_from(cmd.args)?)),
                Some(me::Schema::RequestSchema(args)) =>
                    Ok(Request::MeRequestSchema(me::request_schema::Request::try_from(args)?)),
                Some(me::Schema::ResponseSchema(args)) =>
                    Ok(Request::MeResponseSchema(me::response_schema::Request::try_from(args)?)),
            },
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) =>
                    Ok(Request::ListRequestSchema(list::request_schema::Request::try_from(args)?)),
                Some(list::Schema::ResponseSchema(args)) =>
                    Ok(Request::ListResponseSchema(list::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Spawn(inner) => inner.into_command(),
            Request::SpawnRequestSchema(inner) => inner.into_command(),
            Request::SpawnResponseSchema(inner) => inner.into_command(),
            Request::Message(inner) => inner.into_command(),
            Request::MessageRequestSchema(inner) => inner.into_command(),
            Request::MessageResponseSchema(inner) => inner.into_command(),
            Request::Read(inner) => inner.into_command(),
            Request::Me(inner) => inner.into_command(),
            Request::MeRequestSchema(inner) => inner.into_command(),
            Request::MeResponseSchema(inner) => inner.into_command(),
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
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
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
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
        Request::Read(req) => {
            let inner = read::execute(executor, req, agent_arguments).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Read)))
        }
        Request::Me(req) => {
            let value = me::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Me(value),
            )))
        }
        Request::MeRequestSchema(req) => {
            let value = me::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::MeRequestSchema(value),
            )))
        }
        Request::MeResponseSchema(req) => {
            let value = me::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::MeResponseSchema(value),
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
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>,
    > = match request {
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
        Request::Read(req) => {
            let inner = read::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::Me(req) => {
            let value = me::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::MeRequestSchema(req) => {
            let value =
                me::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::MeResponseSchema(req) => {
            let value =
                me::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::List(req) => {
            let inner = list::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::ListRequestSchema(req) => {
            let value =
                list::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListResponseSchema(req) => {
            let value =
                list::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}
