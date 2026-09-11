//! Opening the watch.

use futures_util::SinkExt as _;
use tokio_tungstenite::tungstenite::Message;

use super::super::request;
use super::{ExecuteError, ExecuteStream};
use crate::server::container_client::{self, ContainerClient};

/// Open `/filesystem/tree`, say what to leave out, and hand back a
/// fresh subscription, starting whole.
///
/// The request is the one message sent. The stream yields the
/// snapshot and then every change for as long as it is held; dropping
/// it closes the socket, which the proxy treats as the server
/// leaving, and a second call is a second watch. A watch the proxy
/// could not begin arrives as the stream's one error.
pub async fn execute(client: &ContainerClient, request: &request::Request) -> Result<ExecuteStream, ExecuteError> {
    let mut socket = client.open("/filesystem/tree").await.map_err(ExecuteError::Open)?;
    let bytes = container_client::encoded(request).map_err(ExecuteError::Encode)?;
    socket
        .send(Message::Binary(bytes))
        .await
        .map_err(ExecuteError::Socket)?;
    Ok(ExecuteStream::new(socket))
}
