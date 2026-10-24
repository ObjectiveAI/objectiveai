//! A connect, one MCP exchange through it, and the disconnect.

use std::fmt::Display;

use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::execute::{
    self, ExecuteError, ExecuteHandle,
};
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use diverge_sdk::wire::client::handle::Handle;
use serde::Serialize;

use super::Answer;
use crate::run::{Emitter, Error};

/// What a connect leaf prints, one line: the connect's own answer when
/// the daemon refuses it, or the exchange's result as the MCP result's
/// own JSON, bare — untagged, so the line is the one or the other as
/// it is.
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Line<T> {
    /// The connect refused: see [`Answer`].
    Answer(Answer),
    /// The exchange's result.
    Result(T),
}

/// The tool connected to, the one `exchange` put through the scope,
/// its result printed as a [`Line`], and the scope disconnected and
/// waited to its end. A connect the daemon refuses — no such tool,
/// forbidden, or its own error — is printed as the answer it is, and
/// the run ends there. Which exchange, and with what, is the leaf's:
/// the shape holds the scope around it.
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
        Err(ExecuteError::NotFound) => {
            return emit(emitter, Line::<T>::Answer(Answer::NotFound)).await;
        }
        Err(ExecuteError::Forbidden) => {
            return emit(emitter, Line::<T>::Answer(Answer::Forbidden)).await;
        }
        Err(ExecuteError::Daemon(error)) => {
            return emit(emitter, Line::<T>::Answer(Answer::Error(error))).await;
        }
        Err(ExecuteError::Closed | ExecuteError::Send(_)) => return Err(Error::Rejected),
        Err(error) => return Err(Error::Exchange(error.to_string())),
    };
    let outcome = exchange(&tool).await;
    let _ = tool.disconnect().await;
    tool.wait().await;
    let result = outcome.map_err(|error| Error::Exchange(error.to_string()))?;
    emit(emitter, Line::Result(result)).await
}

/// One line printed.
async fn emit<T: Serialize>(emitter: &mut Emitter, line: Line<T>) -> Result<(), Error> {
    emitter.emit(&line).await.map(|_| ())
}
