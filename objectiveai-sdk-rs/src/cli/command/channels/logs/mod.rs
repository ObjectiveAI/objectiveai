//! `channels logs` — the per-channel append-only message log. Leaves:
//!
//! - `request` — publisher→owner write.
//! - `reply` — owner→publisher write.
//! - `list --all | --pending` — envelope listing (no content).
//! - `open --entry-id <id>` — reveal one entry's content.
//! - `subscribe` — long-poll for new entries / channel close.

use crate::cli::command::CommandRequest;

pub mod list;
pub mod open;
pub mod reply;
pub mod request;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Publisher→owner write.
    Request(request::Command),
    /// Owner→publisher write.
    Reply(reply::Command),
    /// List the channel log as envelopes (`--all` / `--pending`).
    List(list::Command),
    /// Reveal one entry's content by `--entry-id`.
    Open(open::Command),
    /// Long-poll for new entries / channel close.
    Subscribe(subscribe::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.channels.logs.Request")]
pub enum Request {
    #[schemars(title = "Request")]
    Request(request::Request),
    #[schemars(title = "RequestRequestSchema")]
    RequestRequestSchema(request::request_schema::Request),
    #[schemars(title = "RequestResponseSchema")]
    RequestResponseSchema(request::response_schema::Request),
    #[schemars(title = "Reply")]
    Reply(reply::Request),
    #[schemars(title = "ReplyRequestSchema")]
    ReplyRequestSchema(reply::request_schema::Request),
    #[schemars(title = "ReplyResponseSchema")]
    ReplyResponseSchema(reply::response_schema::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
    #[schemars(title = "Open")]
    Open(open::Request),
    #[schemars(title = "OpenRequestSchema")]
    OpenRequestSchema(open::request_schema::Request),
    #[schemars(title = "OpenResponseSchema")]
    OpenResponseSchema(open::response_schema::Request),
    #[schemars(title = "Subscribe")]
    Subscribe(subscribe::Request),
    #[schemars(title = "SubscribeRequestSchema")]
    SubscribeRequestSchema(subscribe::request_schema::Request),
    #[schemars(title = "SubscribeResponseSchema")]
    SubscribeResponseSchema(subscribe::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.channels.logs.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Request")]
    Request(request::Response),
    #[schemars(title = "RequestRequestSchema")]
    RequestRequestSchema(request::request_schema::Response),
    #[schemars(title = "RequestResponseSchema")]
    RequestResponseSchema(request::response_schema::Response),
    #[schemars(title = "Reply")]
    Reply(reply::Response),
    #[schemars(title = "ReplyRequestSchema")]
    ReplyRequestSchema(reply::request_schema::Response),
    #[schemars(title = "ReplyResponseSchema")]
    ReplyResponseSchema(reply::response_schema::Response),
    #[schemars(title = "List")]
    List(list::Response),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
    #[schemars(title = "Open")]
    Open(open::Response),
    #[schemars(title = "OpenRequestSchema")]
    OpenRequestSchema(open::request_schema::Response),
    #[schemars(title = "OpenResponseSchema")]
    OpenResponseSchema(open::response_schema::Response),
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
            ResponseItem::Request(v) => v.into_mcp(),
            ResponseItem::RequestRequestSchema(v) => v.into_mcp(),
            ResponseItem::RequestResponseSchema(v) => v.into_mcp(),
            ResponseItem::Reply(v) => v.into_mcp(),
            ResponseItem::ReplyRequestSchema(v) => v.into_mcp(),
            ResponseItem::ReplyResponseSchema(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
            ResponseItem::Open(v) => v.into_mcp(),
            ResponseItem::OpenRequestSchema(v) => v.into_mcp(),
            ResponseItem::OpenResponseSchema(v) => v.into_mcp(),
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
            Command::Request(cmd) => match cmd.schema {
                None => Ok(Request::Request(request::Request::try_from(cmd.args)?)),
                Some(request::Schema::RequestSchema(args)) => Ok(
                    Request::RequestRequestSchema(request::request_schema::Request::try_from(args)?),
                ),
                Some(request::Schema::ResponseSchema(args)) => Ok(
                    Request::RequestResponseSchema(request::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Reply(cmd) => match cmd.schema {
                None => Ok(Request::Reply(reply::Request::try_from(cmd.args)?)),
                Some(reply::Schema::RequestSchema(args)) => Ok(
                    Request::ReplyRequestSchema(reply::request_schema::Request::try_from(args)?),
                ),
                Some(reply::Schema::ResponseSchema(args)) => Ok(
                    Request::ReplyResponseSchema(reply::response_schema::Request::try_from(args)?),
                ),
            },
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) => Ok(
                    Request::ListRequestSchema(list::request_schema::Request::try_from(args)?),
                ),
                Some(list::Schema::ResponseSchema(args)) => Ok(
                    Request::ListResponseSchema(list::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Open(cmd) => match cmd.schema {
                None => Ok(Request::Open(open::Request::try_from(cmd.args)?)),
                Some(open::Schema::RequestSchema(args)) => Ok(
                    Request::OpenRequestSchema(open::request_schema::Request::try_from(args)?),
                ),
                Some(open::Schema::ResponseSchema(args)) => Ok(
                    Request::OpenResponseSchema(open::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Subscribe(cmd) => match cmd.schema {
                None => Ok(Request::Subscribe(subscribe::Request::try_from(cmd.args)?)),
                Some(subscribe::Schema::RequestSchema(args)) => Ok(
                    Request::SubscribeRequestSchema(subscribe::request_schema::Request::try_from(args)?),
                ),
                Some(subscribe::Schema::ResponseSchema(args)) => Ok(
                    Request::SubscribeResponseSchema(subscribe::response_schema::Request::try_from(args)?),
                ),
            },
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Request(inner) => inner.request_base(),
            Request::RequestRequestSchema(inner) => inner.request_base(),
            Request::RequestResponseSchema(inner) => inner.request_base(),
            Request::Reply(inner) => inner.request_base(),
            Request::ReplyRequestSchema(inner) => inner.request_base(),
            Request::ReplyResponseSchema(inner) => inner.request_base(),
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
            Request::Open(inner) => inner.request_base(),
            Request::OpenRequestSchema(inner) => inner.request_base(),
            Request::OpenResponseSchema(inner) => inner.request_base(),
            Request::Subscribe(inner) => inner.request_base(),
            Request::SubscribeRequestSchema(inner) => inner.request_base(),
            Request::SubscribeResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Request(inner) => inner.request_base_mut(),
            Request::RequestRequestSchema(inner) => inner.request_base_mut(),
            Request::RequestResponseSchema(inner) => inner.request_base_mut(),
            Request::Reply(inner) => inner.request_base_mut(),
            Request::ReplyRequestSchema(inner) => inner.request_base_mut(),
            Request::ReplyResponseSchema(inner) => inner.request_base_mut(),
            Request::List(inner) => inner.request_base_mut(),
            Request::ListRequestSchema(inner) => inner.request_base_mut(),
            Request::ListResponseSchema(inner) => inner.request_base_mut(),
            Request::Open(inner) => inner.request_base_mut(),
            Request::OpenRequestSchema(inner) => inner.request_base_mut(),
            Request::OpenResponseSchema(inner) => inner.request_base_mut(),
            Request::Subscribe(inner) => inner.request_base_mut(),
            Request::SubscribeRequestSchema(inner) => inner.request_base_mut(),
            Request::SubscribeResponseSchema(inner) => inner.request_base_mut(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    identity: Option<&crate::identity::Identity>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
        Request::Request(req) => {
            let value = request::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Request(value))))
        }
        Request::RequestRequestSchema(req) => {
            let value = request::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::RequestRequestSchema(value))))
        }
        Request::RequestResponseSchema(req) => {
            let value = request::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::RequestResponseSchema(value))))
        }
        Request::Reply(req) => {
            let value = reply::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Reply(value))))
        }
        Request::ReplyRequestSchema(req) => {
            let value = reply::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::ReplyRequestSchema(value))))
        }
        Request::ReplyResponseSchema(req) => {
            let value = reply::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::ReplyResponseSchema(value))))
        }
        Request::List(req) => {
            let value = list::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::List(value))))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::ListRequestSchema(value))))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::ListResponseSchema(value))))
        }
        Request::Open(req) => {
            let value = open::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Open(value))))
        }
        Request::OpenRequestSchema(req) => {
            let value = open::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::OpenRequestSchema(value))))
        }
        Request::OpenResponseSchema(req) => {
            let value = open::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::OpenResponseSchema(value))))
        }
        Request::Subscribe(req) => {
            let inner = subscribe::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Subscribe)))
        }
        Request::SubscribeRequestSchema(req) => {
            let value = subscribe::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::SubscribeRequestSchema(value))))
        }
        Request::SubscribeResponseSchema(req) => {
            let value = subscribe::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::SubscribeResponseSchema(value))))
        }
    };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    transform: crate::cli::command::Transform,
    identity: Option<&crate::identity::Identity>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>,
    > = match request {
        Request::Request(req) => {
            let value = request::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::RequestRequestSchema(req) => {
            let value = request::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::RequestResponseSchema(req) => {
            let value = request::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Reply(req) => {
            let value = reply::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ReplyRequestSchema(req) => {
            let value = reply::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ReplyResponseSchema(req) => {
            let value = reply::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::List(req) => {
            let value = list::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Open(req) => {
            let value = open::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::OpenRequestSchema(req) => {
            let value = open::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::OpenResponseSchema(req) => {
            let value = open::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Subscribe(req) => {
            let inner = subscribe::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
        Request::SubscribeRequestSchema(req) => {
            let value = subscribe::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::SubscribeResponseSchema(req) => {
            let value = subscribe::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Request(request::ListenerExecution),
    RequestRequestSchema(request::request_schema::ListenerExecution),
    RequestResponseSchema(request::response_schema::ListenerExecution),
    Reply(reply::ListenerExecution),
    ReplyRequestSchema(reply::request_schema::ListenerExecution),
    ReplyResponseSchema(reply::response_schema::ListenerExecution),
    List(list::ListenerExecution),
    ListRequestSchema(list::request_schema::ListenerExecution),
    ListResponseSchema(list::response_schema::ListenerExecution),
    Open(open::ListenerExecution),
    OpenRequestSchema(open::request_schema::ListenerExecution),
    OpenResponseSchema(open::response_schema::ListenerExecution),
    Subscribe(subscribe::ListenerExecution),
    SubscribeRequestSchema(subscribe::request_schema::ListenerExecution),
    SubscribeResponseSchema(subscribe::response_schema::ListenerExecution),
}
