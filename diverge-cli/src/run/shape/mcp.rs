//! A connect, one MCP exchange through it, and the disconnect.

use std::fmt::Display;

use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::execute::{
    self, ExecuteError, ExecuteHandle,
};
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use diverge_sdk::wire::client::handle::Handle;
use serde::Serialize;
use serde_json::Value;

use crate::run::{Emitter, Error};

/// The tool connected to, the one `exchange` put through the scope,
/// its result printed as the MCP result's own JSON, and the scope
/// disconnected and waited to its end. A connect the daemon refuses —
/// no such tool, forbidden, or its own error — is printed as the
/// answer it is, and the run ends there. Which exchange, and with
/// what, is the leaf's: the shape holds the scope around it.
pub async fn mcp<F, T, E>(
    handle: &Handle,
    connect: &request::Frame,
    exchange: F,
    emitter: &mut Emitter,
) -> Result<(), Error>
where
    F: AsyncFnOnce(&ExecuteHandle) -> Result<T, E>,
    T: Serialize,
    E: Display,
{
    let tool = match execute::execute(handle, connect).await {
        Ok(tool) => tool,
        Err(ExecuteError::NotFound) => return emitter.emit_value(Value::from("not_found")).await,
        Err(ExecuteError::Forbidden) => return emitter.emit_value(Value::from("forbidden")).await,
        Err(ExecuteError::Daemon(error)) => {
            return emitter
                .emit_value(serde_json::json!({ "error": error }))
                .await;
        }
        Err(ExecuteError::Closed | ExecuteError::Send(_)) => return Err(Error::Rejected),
        Err(error) => return Err(Error::Exchange(error.to_string())),
    };
    let outcome = answer(exchange(&tool).await);
    let _ = tool.disconnect().await;
    tool.wait().await;
    emitter.emit_value(outcome?).await
}

/// The exchange's result as JSON, or its failure in the SDK's words.
fn answer<T: Serialize, E: Display>(outcome: Result<T, E>) -> Result<Value, Error> {
    let result = outcome.map_err(|error| Error::Exchange(error.to_string()))?;
    serde_json::to_value(result).map_err(Error::Json)
}
