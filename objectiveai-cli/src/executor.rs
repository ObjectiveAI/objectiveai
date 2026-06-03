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
    CommandExecutor, CommandRequest, CommandResponse, parse_request,
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

impl CommandExecutor for CliCommandExecutor {
    type Error = Error;
    type Stream<T>
        = Pin<Box<dyn Stream<Item = Result<T, Self::Error>> + Send>>
    where
        T: Send + 'static;

    async fn execute<R, T>(&self, request: R) -> Result<Self::Stream<T>, Self::Error>
    where
        R: CommandRequest + Send,
        T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
    {
        let argv = request.into_command();
        let sdk_request = parse_request(&argv).map_err(|e| match e {
            objectiveai_sdk::cli::command::ParseError::Clap(e) => Error::ClapParse(e),
            objectiveai_sdk::cli::command::ParseError::FromArgs(e) => Error::FromArgs(e),
        })?;
        let stream = crate::command::command::execute(&self.ctx, sdk_request).await?;
        let mapped = stream.map(|r| {
            r.and_then(|item| {
                let value = serde_json::to_value(item).map_err(Error::InlineJson)?;
                extract_leaf::<T>(value).map_err(Error::InlineJson)
            })
        });
        Ok(Box::pin(mapped))
    }

    async fn execute_one<R, T>(&self, request: R) -> Result<T, Self::Error>
    where
        R: CommandRequest + Send,
        T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut stream = self.execute::<R, T>(request).await?;
        match stream.next().await {
            Some(item) => item,
            None => Err(Error::EmptyStream),
        }
    }
}
