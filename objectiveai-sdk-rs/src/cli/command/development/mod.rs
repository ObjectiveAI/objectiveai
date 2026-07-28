//! `development` — commands that exist for building ObjectiveAI
//! things, not for running them.
//!
//! Everything under here changes how THIS machine behaves while
//! someone is working on a plugin, and none of it is durable: the
//! registrations live in the resident daemon's memory and are gone
//! when it restarts. That is deliberate — a stale development
//! override outliving the work it was for is worse than re-running one
//! command.

use crate::cli::command::CommandRequest;

pub mod plugins;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Development-mode plugin registrations.
    Plugins {
        #[command(subcommand)]
        command: plugins::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.development.Request")]
pub enum Request {
    // NOTE: no variant-level `#[schemars(title)]`. Single-variant enum
    // — schemars collapses it to the lone variant's schema and HOISTS
    // that title over the type's `rename`, which the json-schema
    // builder rejects as a title changing during normalization. Same
    // reason as `agent.script.Script` / `ClientLaboratoryType`.
    Plugins(plugins::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.development.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Plugins")]
    Plugins(plugins::ResponseItem),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Plugins(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Plugins { command } => {
                Ok(Request::Plugins(plugins::Request::try_from(command)?))
            }
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Plugins(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Plugins(inner) => inner.request_base_mut(),
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
        Request::Plugins(req) => {
            let inner = plugins::execute(executor, req, identity).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Plugins)))
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
        Request::Plugins(req) => {
            let inner = plugins::execute_transform(executor, req, transform, identity).await?;
            Box::pin(inner)
        }
    };
    Ok(stream)
}

/// `/listen` mirror of [`Request`]. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub enum ListenerExecution {
    Plugins(plugins::ListenerExecution),
}
