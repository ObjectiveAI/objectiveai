//! One file read out of a volume, one written into it, at rest.

use bytes::Bytes;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::volumes::read::client::{execute as read_execute, request as read_request};
use diverge_sdk::provider::endpoints::volumes::write::client::{execute as write_execute, request as write_request};
use diverge_sdk::shared::error::Error;
use futures_util::StreamExt as _;

use super::{Fail, provider};
use crate::content::Pieces;
use crate::daemon::Daemon;

/// The file at `path` in the volume, as the provider reads it at
/// rest: its pieces, and the provider's refusal as the stream's one
/// error. A provider that refuses before any piece is the `Fail`.
pub async fn read(daemon: &Daemon, volume: &reference::Volume, path: &[String]) -> Result<Pieces, Fail> {
    let handle = provider::handle(daemon, &volume.provider).await?;
    let request = read_request::Frame {
        name: volume.name.clone(),
        path: path.to_vec(),
    };
    let stream = read_execute::execute(&handle, &request)
        .await
        .map_err(|error| Fail::failed(&error))?;
    Ok(Box::pin(stream.map(|piece: Result<Bytes, _>| match piece {
        Ok(bytes) => Ok(bytes),
        Err(read_execute::ExecuteStreamError::Refused(error)) => Err(super::describe(&error)),
        Err(error) => Err(error.to_string()),
    })))
}

/// The file at `path` in the volume replaced whole by `content`,
/// every parent made, as the provider writes it at rest; content that
/// ends in an error is a write the provider abandons.
pub async fn write(daemon: &Daemon, volume: &reference::Volume, path: &[String], content: Pieces) -> Result<(), Fail> {
    let handle = provider::handle(daemon, &volume.provider).await?;
    let request = write_request::Frame {
        name: volume.name.clone(),
        path: path.to_vec(),
    };
    let content = content.map(|piece| piece.map_err(|why| Error(serde_json::json!({"kind": "source", "error": why}))));
    match write_execute::execute(&handle, &request, content).await {
        Ok(()) => Ok(()),
        Err(write_execute::ExecuteError::Refused(error)) => Err(Fail::refusal(&error)),
        Err(write_execute::ExecuteError::Source(error)) => Err(Fail::Error(super::describe(&error))),
        Err(error) => Err(Fail::failed(&error)),
    }
}
