//! Asking the container's server.

use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;

use super::super::{request, response};
use super::ExecuteError;
use crate::decode::Decode as _;
use crate::server::container_client::{self, ContainerClient};
use crate::server::messages::{MessageError, Messages};

/// Open `/tool/read-resource`, send `request`, and read the one answer.
///
/// The frame comes back whole, its `Error` included: that variant is
/// the server's answer — a refusal with a JSON-RPC code the caller
/// reads — and not a failure of the asking, which is what
/// [`ExecuteError`] is for.
pub async fn execute(
    client: &ContainerClient,
    request: &request::Request,
) -> Result<response::Frame, ExecuteError> {
    let mut socket = client.open("/tool/read-resource").await.map_err(ExecuteError::Open)?;
    let bytes = container_client::encoded(request).map_err(ExecuteError::Encode)?;
    socket
        .send(Message::Binary(bytes))
        .await
        .map_err(ExecuteError::Socket)?;
    let mut messages = Messages::new(socket);
    let answer = match messages.next().await {
        None => return Err(ExecuteError::Unserved),
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
