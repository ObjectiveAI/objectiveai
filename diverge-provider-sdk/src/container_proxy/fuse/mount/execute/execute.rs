//! Making a mount.

use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;

use super::super::{request, response};
use super::ExecuteError;
use crate::server::container_client::{self, ContainerClient};
use crate::server::messages::{MessageError, Messages};

/// Open `/fuse/mount`, send the mount, and wait for the one answer.
///
/// `Ok` is the mount serving at its path;
/// [`Refused`](ExecuteError::Refused) is the proxy's reason it was
/// not made. Neither arrives before the mount is complete, which is
/// what lets a run mount everything and then go on.
pub async fn execute(client: &ContainerClient, request: &request::Request) -> Result<(), ExecuteError> {
    let mut socket = client.open("/fuse/mount").await.map_err(ExecuteError::Open)?;
    let bytes = container_client::encoded(request).map_err(ExecuteError::Encode)?;
    socket.send(Message::Binary(bytes)).await.map_err(ExecuteError::Socket)?;
    let mut messages = Messages::new(socket);
    let answer = match messages.next().await {
        None => return Err(ExecuteError::Unserved),
        Some(Err(MessageError::Socket(error))) => return Err(ExecuteError::Socket(error)),
        Some(Err(MessageError::Closed)) => return Err(ExecuteError::Closed),
        Some(Ok(bytes)) => bytes,
    };
    let frame = response::Frame::decode(&answer).map_err(ExecuteError::Frame)?;
    let outcome = match frame {
        response::Frame::Ok => Ok(()),
        response::Frame::Error(reason) => Err(ExecuteError::Refused(reason.to_string())),
    };
    // The proxy closes after its one message; hear it out.
    if let Some(mut socket) = messages.into_inner() {
        let _ = container_client::drain(&mut socket).await;
    }
    outcome
}
