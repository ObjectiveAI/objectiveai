//! In-process [`CommandExecutor`] implementor backed by the CLI's own
//! leaf handlers. Lets programmatic consumers drive the SDK's per-leaf
//! `execute<E>` entry points against the CLI's local logic without
//! spawning a subprocess.
//!
//! `run.rs` also delegates here for non-`instance` argv — the
//! `objectiveai_sdk::cli::command::execute(&executor, request)` call
//! flows back through this executor down to each leaf.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::{
    AgentArguments, CommandExecutor, CommandRequest, CommandResponse, parse_request,
};
use serde_json::Value;

use crate::context::Context;
use crate::error::Error;

/// In-process executor. Owns a [`Context`] and dispatches each
/// `CommandRequest` through the CLI's local root dispatcher.
pub struct CliCommandExecutor {
    ctx: Context,
}

impl CliCommandExecutor {
    pub fn new(ctx: Context) -> Self {
        Self { ctx }
    }
}

/// Walk down externally-tagged enum wrappers (`{"<Variant>": <inner>}`)
/// until `from_value::<T>` succeeds, or no more single-key-object layers
/// remain. The aggregator's variant keys are PascalCase and leaf fields
/// are snake_case across the SDK, so the first level whose shape matches
/// `T` is the leaf.
fn extract_leaf<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, serde_json::Error> {
    let mut current = value;
    loop {
        match serde_json::from_value::<T>(current.clone()) {
            Ok(t) => return Ok(t),
            Err(_) => match current {
                Value::Object(map) if map.len() == 1 => {
                    current = map.into_iter().next().unwrap().1;
                }
                other => return serde_json::from_value::<T>(other),
            },
        }
    }
}

impl CliCommandExecutor {
    /// Build the per-call [`Context`] for this execute. When
    /// `agent_arguments` is `Some`, clone the base ctx and overwrite
    /// the seven per-request identity fields on its `Config`:
    /// `agent_id`, `agent_full_id`, `agent_remote`, `response_id`,
    /// `response_ids`, `mcp_session_id` are set verbatim (including
    /// `None`, which clears the slot), and `agent_instance_hierarchy`
    /// falls back to `"UNKNOWN"` when missing because it's a non-
    /// nullable String on the cli's `Config`. The clone's API-client
    /// cell is detached afterwards so the memoized `HttpClient`
    /// rebuilds with the overridden identity headers. When
    /// `agent_arguments` is `None`, the base ctx is borrowed
    /// unchanged.
    fn resolve_ctx<'a>(
        &'a self,
        agent_arguments: Option<&AgentArguments>,
    ) -> std::borrow::Cow<'a, Context> {
        match agent_arguments {
            None => std::borrow::Cow::Borrowed(&self.ctx),
            Some(args) => {
                let mut ctx = self.ctx.clone();
                ctx.config.agent_instance_hierarchy = args
                    .agent_instance_hierarchy
                    .clone()
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                ctx.config.agent_id = args.agent_id.clone();
                ctx.config.agent_full_id = args.agent_full_id.clone();
                ctx.config.agent_remote = args.agent_remote.clone();
                ctx.config.response_id = args.response_id.clone();
                ctx.config.response_ids = args.response_ids.clone();
                ctx.config.mcp_session_id = args.mcp_session_id.clone();
                ctx.reset_api_client();
                std::borrow::Cow::Owned(ctx)
            }
        }
    }
}

impl CommandExecutor for CliCommandExecutor {
    type Error = Error;
    type Stream<T>
        = Pin<Box<dyn Stream<Item = Result<T, Self::Error>> + Send>>
    where
        T: Send + 'static;

    async fn execute<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<Self::Stream<T>, Self::Error>
    where
        R: CommandRequest + Send,
        T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
    {
        let argv = request.into_command();
        let sdk_request = parse_request(&argv).map_err(|e| match e {
            objectiveai_sdk::cli::command::ParseError::Clap(e) => Error::ClapParse(e),
            objectiveai_sdk::cli::command::ParseError::FromArgs(e) => Error::FromArgs(e),
        })?;
        let ctx = self.resolve_ctx(agent_arguments);
        let stream = crate::command::command::execute(&ctx, sdk_request).await?;
        let mapped = stream.map(|r| {
            r.and_then(|item| {
                let value = serde_json::to_value(item).map_err(Error::InlineJson)?;
                extract_leaf::<T>(value).map_err(Error::InlineJson)
            })
        });
        Ok(Box::pin(mapped))
    }

    async fn execute_one<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<T, Self::Error>
    where
        R: CommandRequest + Send,
        T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut stream: Self::Stream<T> =
            self.execute::<R, T>(request, agent_arguments).await?;
        match stream.next().await {
            Some(item) => item,
            None => Err(Error::EmptyStream),
        }
    }
}
