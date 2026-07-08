//! `laboratories` — top-level group for laboratory containers (podman
//! containers the conduit dials as client-side MCP servers), sibling to
//! `agents`/`swarms`. `create` creates + starts a laboratory container;
//! `list` streams the laboratory containers created in this state;
//! `attach`/`detach` record/remove a laboratory id on an agent target
//! (a tag, or an instance hierarchy). Read attachments back via
//! `agents instances get` (the `laboratories` field).

use crate::cli::command::CommandRequest;

pub mod attach;
pub mod create;
pub mod detach;
pub mod list;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Attach a laboratory id to an agent target.
    Attach(attach::Command),
    /// Create + start a laboratory container.
    Create(create::Command),
    /// Detach a laboratory id from an agent target.
    Detach(detach::Command),
    /// List the laboratory containers created in this state.
    List(list::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.laboratories.Request")]
pub enum Request {
    #[schemars(title = "Attach")]
    Attach(attach::Request),
    #[schemars(title = "AttachRequestSchema")]
    AttachRequestSchema(attach::request_schema::Request),
    #[schemars(title = "AttachResponseSchema")]
    AttachResponseSchema(attach::response_schema::Request),
    #[schemars(title = "Create")]
    Create(create::Request),
    #[schemars(title = "CreateRequestSchema")]
    CreateRequestSchema(create::request_schema::Request),
    #[schemars(title = "CreateResponseSchema")]
    CreateResponseSchema(create::response_schema::Request),
    #[schemars(title = "Detach")]
    Detach(detach::Request),
    #[schemars(title = "DetachRequestSchema")]
    DetachRequestSchema(detach::request_schema::Request),
    #[schemars(title = "DetachResponseSchema")]
    DetachResponseSchema(detach::response_schema::Request),
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
#[schemars(rename = "cli.command.laboratories.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Attach")]
    Attach(attach::Response),
    #[schemars(title = "AttachRequestSchema")]
    AttachRequestSchema(attach::request_schema::Response),
    #[schemars(title = "AttachResponseSchema")]
    AttachResponseSchema(attach::response_schema::Response),
    #[schemars(title = "Create")]
    Create(create::Response),
    #[schemars(title = "CreateRequestSchema")]
    CreateRequestSchema(create::request_schema::Response),
    #[schemars(title = "CreateResponseSchema")]
    CreateResponseSchema(create::response_schema::Response),
    #[schemars(title = "Detach")]
    Detach(detach::Response),
    #[schemars(title = "DetachRequestSchema")]
    DetachRequestSchema(detach::request_schema::Response),
    #[schemars(title = "DetachResponseSchema")]
    DetachResponseSchema(detach::response_schema::Response),
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
            ResponseItem::Attach(v) => v.into_mcp(),
            ResponseItem::AttachRequestSchema(v) => v.into_mcp(),
            ResponseItem::AttachResponseSchema(v) => v.into_mcp(),
            ResponseItem::Create(v) => v.into_mcp(),
            ResponseItem::CreateRequestSchema(v) => v.into_mcp(),
            ResponseItem::CreateResponseSchema(v) => v.into_mcp(),
            ResponseItem::Detach(v) => v.into_mcp(),
            ResponseItem::DetachRequestSchema(v) => v.into_mcp(),
            ResponseItem::DetachResponseSchema(v) => v.into_mcp(),
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
            Command::Attach(cmd) => match cmd.schema {
                None => Ok(Request::Attach(attach::Request::try_from(cmd.args)?)),
                Some(attach::Schema::RequestSchema(args)) => Ok(Request::AttachRequestSchema(
                    attach::request_schema::Request::try_from(args)?,
                )),
                Some(attach::Schema::ResponseSchema(args)) => Ok(Request::AttachResponseSchema(
                    attach::response_schema::Request::try_from(args)?,
                )),
            },
            Command::Create(cmd) => match cmd.schema {
                None => Ok(Request::Create(create::Request::try_from(cmd.args)?)),
                Some(create::Schema::RequestSchema(args)) => Ok(Request::CreateRequestSchema(
                    create::request_schema::Request::try_from(args)?,
                )),
                Some(create::Schema::ResponseSchema(args)) => Ok(Request::CreateResponseSchema(
                    create::response_schema::Request::try_from(args)?,
                )),
            },
            Command::Detach(cmd) => match cmd.schema {
                None => Ok(Request::Detach(detach::Request::try_from(cmd.args)?)),
                Some(detach::Schema::RequestSchema(args)) => Ok(Request::DetachRequestSchema(
                    detach::request_schema::Request::try_from(args)?,
                )),
                Some(detach::Schema::ResponseSchema(args)) => Ok(Request::DetachResponseSchema(
                    detach::response_schema::Request::try_from(args)?,
                )),
            },
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) => Ok(Request::ListRequestSchema(
                    list::request_schema::Request::try_from(args)?,
                )),
                Some(list::Schema::ResponseSchema(args)) => Ok(Request::ListResponseSchema(
                    list::response_schema::Request::try_from(args)?,
                )),
            },
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Attach(inner) => inner.request_base(),
            Request::AttachRequestSchema(inner) => inner.request_base(),
            Request::AttachResponseSchema(inner) => inner.request_base(),
            Request::Create(inner) => inner.request_base(),
            Request::CreateRequestSchema(inner) => inner.request_base(),
            Request::CreateResponseSchema(inner) => inner.request_base(),
            Request::Detach(inner) => inner.request_base(),
            Request::DetachRequestSchema(inner) => inner.request_base(),
            Request::DetachResponseSchema(inner) => inner.request_base(),
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Attach(inner) => inner.request_base_mut(),
            Request::AttachRequestSchema(inner) => inner.request_base_mut(),
            Request::AttachResponseSchema(inner) => inner.request_base_mut(),
            Request::Create(inner) => inner.request_base_mut(),
            Request::CreateRequestSchema(inner) => inner.request_base_mut(),
            Request::CreateResponseSchema(inner) => inner.request_base_mut(),
            Request::Detach(inner) => inner.request_base_mut(),
            Request::DetachRequestSchema(inner) => inner.request_base_mut(),
            Request::DetachResponseSchema(inner) => inner.request_base_mut(),
            Request::List(inner) => inner.request_base_mut(),
            Request::ListRequestSchema(inner) => inner.request_base_mut(),
            Request::ListResponseSchema(inner) => inner.request_base_mut(),
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
    use futures::StreamExt as _;
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
        Request::Attach(req) => {
            let value = attach::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Attach(value),
            )))
        }
        Request::AttachRequestSchema(req) => {
            let value = attach::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::AttachRequestSchema(value),
            )))
        }
        Request::AttachResponseSchema(req) => {
            let value = attach::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::AttachResponseSchema(value),
            )))
        }
        Request::Create(req) => {
            let value = create::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Create(value),
            )))
        }
        Request::CreateRequestSchema(req) => {
            let value = create::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::CreateRequestSchema(value),
            )))
        }
        Request::CreateResponseSchema(req) => {
            let value = create::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::CreateResponseSchema(value),
            )))
        }
        Request::Detach(req) => {
            let value = detach::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::Detach(value),
            )))
        }
        Request::DetachRequestSchema(req) => {
            let value = detach::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::DetachRequestSchema(value),
            )))
        }
        Request::DetachResponseSchema(req) => {
            let value = detach::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::DetachResponseSchema(value),
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
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    transform: crate::cli::command::Transform,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>,
    > = match request {
        Request::Attach(req) => {
            let value = attach::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::AttachRequestSchema(req) => {
            let value = attach::request_schema::execute_transform(
                executor,
                req,
                transform,
                agent_arguments,
            )
            .await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::AttachResponseSchema(req) => {
            let value = attach::response_schema::execute_transform(
                executor,
                req,
                transform,
                agent_arguments,
            )
            .await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Create(req) => {
            let value = create::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::CreateRequestSchema(req) => {
            let value = create::request_schema::execute_transform(
                executor,
                req,
                transform,
                agent_arguments,
            )
            .await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::CreateResponseSchema(req) => {
            let value = create::response_schema::execute_transform(
                executor,
                req,
                transform,
                agent_arguments,
            )
            .await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Detach(req) => {
            let value = detach::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DetachRequestSchema(req) => {
            let value = detach::request_schema::execute_transform(
                executor,
                req,
                transform,
                agent_arguments,
            )
            .await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::DetachResponseSchema(req) => {
            let value = detach::response_schema::execute_transform(
                executor,
                req,
                transform,
                agent_arguments,
            )
            .await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::List(req) => {
            let inner = list::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute_transform(
                executor,
                req,
                transform,
                agent_arguments,
            )
            .await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute_transform(
                executor,
                req,
                transform,
                agent_arguments,
            )
            .await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Attach(attach::ListenerExecution),
    AttachRequestSchema(attach::request_schema::ListenerExecution),
    AttachResponseSchema(attach::response_schema::ListenerExecution),
    Create(create::ListenerExecution),
    CreateRequestSchema(create::request_schema::ListenerExecution),
    CreateResponseSchema(create::response_schema::ListenerExecution),
    Detach(detach::ListenerExecution),
    DetachRequestSchema(detach::request_schema::ListenerExecution),
    DetachResponseSchema(detach::response_schema::ListenerExecution),
    List(list::ListenerExecution),
    ListRequestSchema(list::request_schema::ListenerExecution),
    ListResponseSchema(list::response_schema::ListenerExecution),
}
