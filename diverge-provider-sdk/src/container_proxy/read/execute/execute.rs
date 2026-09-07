//! Asking for the file.

use futures_util::SinkExt as _;
use tokio_tungstenite::tungstenite::Message;

use super::super::request;
use super::{ExecuteError, ExecuteStream};
use crate::server::container_client::{self, ContainerClient};

/// Open `/read`, name the file, and read it.
///
/// The one message the server sends goes out here; everything after
/// is the stream's. A file the proxy would not read — no such file, a
/// directory, a read that failed partway — arrives as the stream's
/// one error, with the proxy's reason.
pub async fn execute(
    client: &ContainerClient,
    request: &request::Request,
) -> Result<ExecuteStream, ExecuteError> {
    let mut socket = client.open("/read").await.map_err(ExecuteError::Open)?;
    let bytes = container_client::encoded(request).map_err(ExecuteError::Encode)?;
    socket
        .send(Message::Binary(bytes))
        .await
        .map_err(ExecuteError::Socket)?;
    Ok(ExecuteStream::new(socket))
}
