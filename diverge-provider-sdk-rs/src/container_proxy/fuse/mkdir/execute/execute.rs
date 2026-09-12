//! Answering a fuse mkdir.

use futures_util::SinkExt as _;
use tokio_tungstenite::tungstenite::Message;

use super::super::response;
use super::ExecuteError;
use crate::server::container_client::{self, ContainerClient};

/// Answer the ask on `channel`: open `/fuse/mkdir/{channel}`, send
/// `response` if there is one, close cleanly.
///
/// `None` is the wire's refusal — the clean close with no message
/// before it, the container told the ask could not be served. Either
/// way the channel is spent: the proxy answers a second opening with
/// `409`, which arrives as [`Open`](ExecuteError::Open), as does the
/// `404` for a channel it does not know.
pub async fn execute(
    client: &ContainerClient,
    channel: u32,
    response: Option<&response::Frame<'_>>,
) -> Result<(), ExecuteError> {
    let mut socket = client
        .open(&format!("/fuse/mkdir/{channel}"))
        .await
        .map_err(ExecuteError::Open)?;
    if let Some(response) = response {
        let bytes = container_client::encoded(response).unwrap_or_else(|error| match error {});
        socket
            .send(Message::Binary(bytes))
            .await
            .map_err(ExecuteError::Socket)?;
    }
    container_client::finish(socket).await.map_err(ExecuteError::Socket)
}
