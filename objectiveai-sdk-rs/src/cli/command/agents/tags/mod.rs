//! `agents tags` — client-side agent tags. Tags uniquely identify
//! one agent instance hierarchy and are stored locally (not on the
//! API server). Two leaves:
//!
//! - `lookup` — look up by `agent_instance_hierarchy` (returns the
//!   tag, if any) or by `tag` (returns the bound hierarchy, if any).
//! - `remove` — delete a tag registration by name (BOUND or
//!   GROUPED), detaching its laboratory attachments and
//!   garbage-collecting an emptied group.
//! - `apply` — bind a tag to a target. Three target shapes:
//!   - `--me`: BOUND immediately to the cli's own
//!     `Config.agent_instance_hierarchy`.
//!   - `--agent-full-id <id>`: PENDING. The next matching
//!     agent-completion auto-binds the tag on its first chunk.
//!   - `--agent-instance <inst>`: BOUND immediately to
//!     `{parent}/{instance}`.
//!
//!   `--agent-full-id` / `--agent-instance` accept an optional
//!   `--parent-agent-instance-hierarchy` (defaults to ctx own).
//!   `--me` forbids `--parent-agent-instance-hierarchy`.

use crate::cli::command::CommandRequest;

pub mod apply;
pub mod lookup;
pub mod remove;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Resolve a tag → agent-instance-hierarchy or vice versa.
    Lookup(lookup::Command),
    /// Bind a tag to a target (`--me`, `--agent-full-id`, or
    /// `--agent-instance`). Replaces any existing binding silently.
    Apply(apply::Command),
    /// Delete a tag registration by name (whatever its shape),
    /// detaching its laboratory attachments with it.
    Remove(remove::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.tags.Request")]
pub enum Request {
    #[schemars(title = "Lookup")]
    Lookup(lookup::Request),
    #[schemars(title = "LookupRequestSchema")]
    LookupRequestSchema(lookup::request_schema::Request),
    #[schemars(title = "LookupResponseSchema")]
    LookupResponseSchema(lookup::response_schema::Request),
    #[schemars(title = "Apply")]
    Apply(apply::Request),
    #[schemars(title = "ApplyRequestSchema")]
    ApplyRequestSchema(apply::request_schema::Request),
    #[schemars(title = "ApplyResponseSchema")]
    ApplyResponseSchema(apply::response_schema::Request),
    #[schemars(title = "Remove")]
    Remove(remove::Request),
    #[schemars(title = "RemoveRequestSchema")]
    RemoveRequestSchema(remove::request_schema::Request),
    #[schemars(title = "RemoveResponseSchema")]
    RemoveResponseSchema(remove::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Lookup")]
    Lookup(lookup::Response),
    #[schemars(title = "LookupRequestSchema")]
    LookupRequestSchema(lookup::request_schema::Response),
    #[schemars(title = "LookupResponseSchema")]
    LookupResponseSchema(lookup::response_schema::Response),
    #[schemars(title = "Apply")]
    Apply(apply::Response),
    #[schemars(title = "ApplyRequestSchema")]
    ApplyRequestSchema(apply::request_schema::Response),
    #[schemars(title = "ApplyResponseSchema")]
    ApplyResponseSchema(apply::response_schema::Response),
    #[schemars(title = "Remove")]
    Remove(remove::Response),
    #[schemars(title = "RemoveRequestSchema")]
    RemoveRequestSchema(remove::request_schema::Response),
    #[schemars(title = "RemoveResponseSchema")]
    RemoveResponseSchema(remove::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Lookup(v) => v.into_mcp(),
            ResponseItem::LookupRequestSchema(v) => v.into_mcp(),
            ResponseItem::LookupResponseSchema(v) => v.into_mcp(),
            ResponseItem::Apply(v) => v.into_mcp(),
            ResponseItem::ApplyRequestSchema(v) => v.into_mcp(),
            ResponseItem::ApplyResponseSchema(v) => v.into_mcp(),
            ResponseItem::Remove(v) => v.into_mcp(),
            ResponseItem::RemoveRequestSchema(v) => v.into_mcp(),
            ResponseItem::RemoveResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Lookup(cmd) => match cmd.schema {
                None => Ok(Request::Lookup(lookup::Request::try_from(cmd.args)?)),
                Some(lookup::Schema::RequestSchema(args)) => Ok(
                    Request::LookupRequestSchema(lookup::request_schema::Request::try_from(args)?),
                ),
                Some(lookup::Schema::ResponseSchema(args)) => Ok(
                    Request::LookupResponseSchema(lookup::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Apply(cmd) => match cmd.schema {
                None => Ok(Request::Apply(apply::Request::try_from(cmd.args)?)),
                Some(apply::Schema::RequestSchema(args)) => Ok(
                    Request::ApplyRequestSchema(apply::request_schema::Request::try_from(args)?),
                ),
                Some(apply::Schema::ResponseSchema(args)) => Ok(
                    Request::ApplyResponseSchema(apply::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Remove(cmd) => match cmd.schema {
                None => Ok(Request::Remove(remove::Request::try_from(cmd.args)?)),
                Some(remove::Schema::RequestSchema(args)) => Ok(
                    Request::RemoveRequestSchema(remove::request_schema::Request::try_from(args)?),
                ),
                Some(remove::Schema::ResponseSchema(args)) => Ok(
                    Request::RemoveResponseSchema(remove::response_schema::Request::try_from(args)?),
                ),
            },
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Lookup(inner) => inner.request_base(),
            Request::LookupRequestSchema(inner) => inner.request_base(),
            Request::LookupResponseSchema(inner) => inner.request_base(),
            Request::Apply(inner) => inner.request_base(),
            Request::ApplyRequestSchema(inner) => inner.request_base(),
            Request::ApplyResponseSchema(inner) => inner.request_base(),
            Request::Remove(inner) => inner.request_base(),
            Request::RemoveRequestSchema(inner) => inner.request_base(),
            Request::RemoveResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Lookup(inner) => inner.request_base_mut(),
            Request::LookupRequestSchema(inner) => inner.request_base_mut(),
            Request::LookupResponseSchema(inner) => inner.request_base_mut(),
            Request::Apply(inner) => inner.request_base_mut(),
            Request::ApplyRequestSchema(inner) => inner.request_base_mut(),
            Request::ApplyResponseSchema(inner) => inner.request_base_mut(),
            Request::Remove(inner) => inner.request_base_mut(),
            Request::RemoveRequestSchema(inner) => inner.request_base_mut(),
            Request::RemoveResponseSchema(inner) => inner.request_base_mut(),
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
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
        Request::Lookup(req) => {
            let value = lookup::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Lookup(value))))
        }
        Request::LookupRequestSchema(req) => {
            let value = lookup::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::LookupRequestSchema(value),
            )))
        }
        Request::LookupResponseSchema(req) => {
            let value = lookup::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::LookupResponseSchema(value),
            )))
        }
        Request::Apply(req) => {
            let value = apply::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Apply(value))))
        }
        Request::ApplyRequestSchema(req) => {
            let value = apply::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ApplyRequestSchema(value),
            )))
        }
        Request::ApplyResponseSchema(req) => {
            let value = apply::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ApplyResponseSchema(value),
            )))
        }
        Request::Remove(req) => {
            let value = remove::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Remove(value))))
        }
        Request::RemoveRequestSchema(req) => {
            let value = remove::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::RemoveRequestSchema(value),
            )))
        }
        Request::RemoveResponseSchema(req) => {
            let value = remove::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::RemoveResponseSchema(value),
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
        Request::Lookup(req) => {
            let value = lookup::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::LookupRequestSchema(req) => {
            let value =
                lookup::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::LookupResponseSchema(req) => {
            let value =
                lookup::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Apply(req) => {
            let value = apply::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ApplyRequestSchema(req) => {
            let value =
                apply::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ApplyResponseSchema(req) => {
            let value =
                apply::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Remove(req) => {
            let value = remove::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::RemoveRequestSchema(req) => {
            let value =
                remove::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::RemoveResponseSchema(req) => {
            let value =
                remove::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]: one variant per child, wrapping
/// its `ListenerExecution`. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Lookup(lookup::ListenerExecution),
    LookupRequestSchema(lookup::request_schema::ListenerExecution),
    LookupResponseSchema(lookup::response_schema::ListenerExecution),
    Apply(apply::ListenerExecution),
    ApplyRequestSchema(apply::request_schema::ListenerExecution),
    ApplyResponseSchema(apply::response_schema::ListenerExecution),
    Remove(remove::ListenerExecution),
    RemoveRequestSchema(remove::request_schema::ListenerExecution),
    RemoveResponseSchema(remove::response_schema::ListenerExecution),
}
