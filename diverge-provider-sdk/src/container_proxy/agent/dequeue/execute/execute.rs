//! Clearing the queue.

use futures_util::StreamExt as _;

use super::super::response;
use super::ExecuteError;
use crate::decode::Decode as _;
use crate::server::container_client::{self, ContainerClient};
use crate::server::messages::{MessageError, Messages};

/// Open `/agent/dequeue` and read the one answer.
///
/// The frame comes back whole, its `Error` included: that variant is
/// the agent's server's own answer, which the caller's channel takes
/// as it is, and not a failure of this call. What IS a failure of
/// this call is the wire — the path not opening, the answer not
/// decoding, the socket dying.
pub async fn execute(client: &ContainerClient) -> Result<response::Frame, ExecuteError> {
    let socket = client.open("/agent/dequeue").await.map_err(ExecuteError::Open)?;
    let mut messages = Messages::new(socket);
    let answer = match messages.next().await {
        None => return Err(ExecuteError::Unserved),
        Some(Err(MessageError::Text)) => return Err(ExecuteError::Text),
        Some(Err(MessageError::Socket(error))) => return Err(ExecuteError::Socket(error)),
        Some(Err(MessageError::Closed)) => return Err(ExecuteError::Closed),
        Some(Ok(bytes)) => bytes,
    };
    let frame = response::Frame::decode(&answer).map_err(ExecuteError::Frame)?;
    // The proxy closes after its one message; hear it out.
    if let Some(mut socket) = messages.into_inner() {
        let _ = container_client::drain(&mut socket).await;
    }
    Ok(frame)
}
