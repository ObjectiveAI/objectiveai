//! Registering the agent.

use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;

use super::super::{request, response};
use super::ExecuteError;
use crate::decode::Decode as _;
use crate::server::container_client::{self, ContainerClient};
use crate::server::messages::{MessageError, Messages};

/// Open `/agent/register`, send the agent, and read the one answer.
///
/// `Ok` is the container holding the agent for its life;
/// [`Refused`](ExecuteError::Refused) is its own reason — a value the
/// image will not take, or an agent already registered. Once, before
/// the first loop: a second registration is refused whatever it
/// carries.
pub async fn execute(
    client: &ContainerClient,
    request: &request::Request,
) -> Result<(), ExecuteError> {
    let mut socket = client.open("/agent/register").await.map_err(ExecuteError::Open)?;
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
    match frame {
        response::Frame::Registered => Ok(()),
        response::Frame::Error(error) => Err(ExecuteError::Refused(error)),
    }
}
