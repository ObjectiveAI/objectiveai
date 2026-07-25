//! `agents logs` — persisted log tier. Leaves:
//!
//! - `open --id <id>` — look up a single logged row by its
//!   `logs.messages."index"`.
//! - `list --all | --pending` — stream logged rows for the targets:
//!   `--all` is every row, `--pending` is the unfinalized rows only.
//! - `subscribe` — long-lived stream of new rows as they land.

use crate::cli::command::CommandRequest;

pub mod list;
pub mod open;
pub mod subscribe;
pub mod token_usage;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Look up a single logged row by its `logs.messages."index"`.
    Open(open::Command),
    /// Stream logged rows for the targets (`--all` / `--pending`).
    List(list::Command),
    /// Subscribe to live updates for the given agents.
    Subscribe(subscribe::Command),
    /// Per-AIH token-usage snapshot — `subscribe`.
    TokenUsage {
        #[command(subcommand)]
        command: token_usage::Command,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.logs.Request")]
pub enum Request {
    #[schemars(title = "Open")]
    Open(open::Request),
    #[schemars(title = "OpenRequestSchema")]
    OpenRequestSchema(open::request_schema::Request),
    #[schemars(title = "OpenResponseSchema")]
    OpenResponseSchema(open::response_schema::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
    #[schemars(title = "Subscribe")]
    Subscribe(subscribe::Request),
    #[schemars(title = "SubscribeRequestSchema")]
    SubscribeRequestSchema(subscribe::request_schema::Request),
    #[schemars(title = "SubscribeResponseSchema")]
    SubscribeResponseSchema(subscribe::response_schema::Request),
    #[schemars(title = "TokenUsage")]
    TokenUsage(token_usage::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Open")]
    Open(open::Response),
    #[schemars(title = "OpenRequestSchema")]
    OpenRequestSchema(open::request_schema::Response),
    #[schemars(title = "OpenResponseSchema")]
    OpenResponseSchema(open::response_schema::Response),
    #[schemars(title = "List")]
    List(list::ResponseItem),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
    #[schemars(title = "Subscribe")]
    Subscribe(subscribe::ResponseItem),
    #[schemars(title = "SubscribeRequestSchema")]
    SubscribeRequestSchema(subscribe::request_schema::Response),
    #[schemars(title = "SubscribeResponseSchema")]
    SubscribeResponseSchema(subscribe::response_schema::Response),
    #[schemars(title = "TokenUsage")]
    TokenUsage(token_usage::ResponseItem),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Open(v) => v.into_mcp(),
            ResponseItem::OpenRequestSchema(v) => v.into_mcp(),
            ResponseItem::OpenResponseSchema(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
            ResponseItem::Subscribe(v) => v.into_mcp(),
            ResponseItem::SubscribeRequestSchema(v) => v.into_mcp(),
            ResponseItem::SubscribeResponseSchema(v) => v.into_mcp(),
            ResponseItem::TokenUsage(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Open(cmd) => match cmd.schema {
                None => Ok(Request::Open(open::Request::try_from(cmd.args)?)),
                Some(open::Schema::RequestSchema(args)) => Ok(
                    Request::OpenRequestSchema(open::request_schema::Request::try_from(args)?),
                ),
                Some(open::Schema::ResponseSchema(args)) => Ok(
                    Request::OpenResponseSchema(open::response_schema::Request::try_from(args)?),
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
            Command::Subscribe(cmd) => match cmd.schema {
                None => Ok(Request::Subscribe(subscribe::Request::try_from(cmd.args)?)),
                Some(subscribe::Schema::RequestSchema(args)) => Ok(
                    Request::SubscribeRequestSchema(subscribe::request_schema::Request::try_from(args)?),
                ),
                Some(subscribe::Schema::ResponseSchema(args)) => Ok(
                    Request::SubscribeResponseSchema(subscribe::response_schema::Request::try_from(args)?),
                ),
            },
            Command::TokenUsage { command } => {
                Ok(Request::TokenUsage(token_usage::Request::try_from(command)?))
            }
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Open(inner) => inner.request_base(),
            Request::OpenRequestSchema(inner) => inner.request_base(),
            Request::OpenResponseSchema(inner) => inner.request_base(),
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
            Request::Subscribe(inner) => inner.request_base(),
            Request::SubscribeRequestSchema(inner) => inner.request_base(),
            Request::SubscribeResponseSchema(inner) => inner.request_base(),
            Request::TokenUsage(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Open(inner) => inner.request_base_mut(),
            Request::OpenRequestSchema(inner) => inner.request_base_mut(),
            Request::OpenResponseSchema(inner) => inner.request_base_mut(),
            Request::List(inner) => inner.request_base_mut(),
            Request::ListRequestSchema(inner) => inner.request_base_mut(),
            Request::ListResponseSchema(inner) => inner.request_base_mut(),
            Request::Subscribe(inner) => inner.request_base_mut(),
            Request::SubscribeRequestSchema(inner) => inner.request_base_mut(),
            Request::SubscribeResponseSchema(inner) => inner.request_base_mut(),
            Request::TokenUsage(inner) => inner.request_base_mut(),
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
        Request::Open(req) => {
            let value = open::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Open(value),
            )))
        }
        Request::OpenRequestSchema(req) => {
            let value = open::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::OpenRequestSchema(value),
            )))
        }
        Request::OpenResponseSchema(req) => {
            let value = open::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::OpenResponseSchema(value),
            )))
        }
        Request::List(req) => {
            let inner = list::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListRequestSchema(value),
            )))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListResponseSchema(value),
            )))
        }
        Request::Subscribe(req) => {
            let inner = subscribe::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Subscribe)))
        }
        Request::SubscribeRequestSchema(req) => {
            let value = subscribe::request_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::SubscribeRequestSchema(value),
            )))
        }
        Request::SubscribeResponseSchema(req) => {
            let value = subscribe::response_schema::execute(executor, req, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::SubscribeResponseSchema(value),
            )))
        }
        Request::TokenUsage(req) => {
            let inner = token_usage::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::TokenUsage)))
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
        Request::Open(req) => {
            let value = open::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::OpenRequestSchema(req) => {
            let value =
                open::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::OpenResponseSchema(req) => {
            let value =
                open::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::List(req) => {
            let inner = list::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
        Request::ListRequestSchema(req) => {
            let value =
                list::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListResponseSchema(req) => {
            let value =
                list::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Subscribe(req) => {
            let inner = subscribe::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
        Request::SubscribeRequestSchema(req) => {
            let value =
                subscribe::request_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::SubscribeResponseSchema(req) => {
            let value =
                subscribe::response_schema::execute_transform(executor, req, transform, identity).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::TokenUsage(req) => {
            let inner = token_usage::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Open(open::ListenerExecution),
    OpenRequestSchema(open::request_schema::ListenerExecution),
    OpenResponseSchema(open::response_schema::ListenerExecution),
    List(list::ListenerExecution),
    ListRequestSchema(list::request_schema::ListenerExecution),
    ListResponseSchema(list::response_schema::ListenerExecution),
    Subscribe(subscribe::ListenerExecution),
    SubscribeRequestSchema(subscribe::request_schema::ListenerExecution),
    SubscribeResponseSchema(subscribe::response_schema::ListenerExecution),
    TokenUsage(token_usage::ListenerExecution),
}
