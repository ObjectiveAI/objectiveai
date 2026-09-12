//! What the database said, sent back.

use bytes::Bytes;
use futures_util::SinkExt as _;
use futures_util::stream::SplitSink;
use tokio_tungstenite::tungstenite::Message;

use super::HandleError;
use crate::server::container_client::ContainerWebSocket;

/// The database's side of the connection.
///
/// [`send`](Self::send) is pgwire as the database wrote it, one chunk
/// per call; [`finish`](Self::finish) is the database closing, the
/// clean close, after which the proxy shuts the driver's socket.
/// Dropping this without finishing closes the socket abruptly, which
/// the proxy treats the same way — a socket cannot be resumed — so
/// nothing is lost but the clarity, and a server that is done says so.
#[must_use = "dropping the handle ends the connection abruptly"]
pub struct ExecuteHandle {
    sink: SplitSink<ContainerWebSocket, Message>,
}

impl ExecuteHandle {
    pub(super) fn new(sink: SplitSink<ContainerWebSocket, Message>) -> Self {
        ExecuteHandle { sink }
    }

    /// One chunk of what the database said.
    pub async fn send(&mut self, bytes: Bytes) -> Result<(), HandleError> {
        self.sink
            .send(Message::Binary(bytes))
            .await
            .map_err(HandleError::Socket)
    }

    /// The database closed: close cleanly.
    pub async fn finish(mut self) -> Result<(), HandleError> {
        self.sink.close().await.map_err(HandleError::Socket)
    }
}
