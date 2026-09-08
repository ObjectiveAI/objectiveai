//! Starting the loop.

use futures_util::SinkExt as _;
use tokio_tungstenite::tungstenite::Message;

use super::super::request;
use super::{ExecuteError, ExecuteStream};
use crate::server::container_client::{self, ContainerClient};

/// Open `/agent/run`, hand over the request, and read the loop.
///
/// The one message the server sends goes out here; everything after
/// is the stream's. A loop that never ran — the agent refused, the
/// key missing, the history unreadable — arrives as the stream's one
/// error, with the container's reason — a loop already in progress
/// among them, the agent's server's own refusal forwarded.
pub async fn execute(
    client: &ContainerClient,
    request: &request::Request,
) -> Result<ExecuteStream, ExecuteError> {
    let mut socket = client.open("/agent/run").await.map_err(ExecuteError::Open)?;
    let bytes = container_client::encoded(request).map_err(ExecuteError::Encode)?;
    socket
        .send(Message::Binary(bytes))
        .await
        .map_err(ExecuteError::Socket)?;
    Ok(ExecuteStream::new(socket))
}
