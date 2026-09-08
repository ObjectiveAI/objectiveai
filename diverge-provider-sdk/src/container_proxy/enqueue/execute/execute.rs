//! Enqueuing a message.

use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;

use super::super::{request, response};
use super::ExecuteError;
use crate::decode::Decode as _;
use crate::server::container_client::{self, ContainerClient};
use crate::server::messages::{MessageError, Messages};

/// Open `/agent/enqueue`, send the message, and wait for its fate.
///
/// Returns when the fate is known, however long that takes — nothing
/// times anything out. The frame comes back whole, its `Error`
/// included: that variant is the agent's server's own answer, which
/// the caller's channel takes as it is, and not a failure of this
/// call. What IS a failure of this call is the wire — the path not
/// opening, the answer not decoding, the socket dying.
pub async fn execute(
    client: &ContainerClient,
    request: &request::Request,
) -> Result<response::Frame, ExecuteError> {
    let mut socket = client.open("/agent/enqueue").await.map_err(ExecuteError::Open)?;
    let bytes = container_client::encoded(request).map_err(ExecuteError::Encode)?;
    socket.send(Message::Binary(bytes)).await.map_err(ExecuteError::Socket)?;
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
