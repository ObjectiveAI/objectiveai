//! Joining a tool container.

use std::fmt;
use std::sync::Arc;

use super::super::{channel_request, channel_response, request};
use super::execute_handle::ExecuteHandle;
use super::serve;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::containers::client::{Encoders, Scoped, Writes, encoded};
use crate::shared::containers::postgres;
use crate::shared::containers::write_bytes;
use crate::shared::error::Error;

/// Join a tool container somebody else is running.
///
/// The request goes out and this returns: the main stream says
/// nothing on success, so there is no frame to wait for. The scope is
/// open — or refused — from here on, and the refusal, or the
/// connection's end, is what [`ExecuteHandle::wait`] reports. A task
/// reads the one kind of channel the provider opens on a connector,
/// the content of its own writes, and answers it.
///
/// # It fails two ways
///
/// The request either serializes and goes out, or it does not.
/// Everything after that is the connection's, and the handle reports
/// it: a container that was not there or a runner that said no is the
/// provider's own error frame, then the finish, on `wait`.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<ExecuteHandle, ExecuteError> {
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    let writes = Arc::new(Writes::new());
    tokio::spawn(serve::serve(
        scope.request_receiver,
        handle.clone(),
        scope.scope,
        Arc::clone(&writes),
        ENCODERS,
    ));
    let scoped = Scoped::new(handle.clone(), scope.scope, writes, scope.response_receiver);
    Ok(ExecuteHandle::new(Arc::new(scoped)))
}

/// This family's frames, for the shared answers. A connector is
/// never asked to carry a database connection — the container's asks
/// go to its runner — so the half it would open is never sent.
const ENCODERS: Encoders = Encoders {
    postgres_half,
    write_body,
    write_error,
};

fn postgres_half(connection_id: u32) -> Option<Vec<u8>> {
    encoded(&channel_request::Frame::Postgres(postgres::request::Postgres { connection_id }))
}

fn write_body(piece: &[u8]) -> Option<Vec<u8>> {
    encoded(&channel_response::write_bytes::Frame::Body(write_bytes::response::Frame(piece)))
}

fn write_error(error: &Error) -> Option<Vec<u8>> {
    encoded(&channel_response::write_bytes::Frame::Error(error.clone()))
}

/// A connection that never started.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Request(error) => {
                write!(f, "tools connect request did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
        }
    }
}
