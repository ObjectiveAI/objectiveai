//! The path, open, and what the server says on it.

use futures_util::SinkExt as _;
use tokio_tungstenite::tungstenite::Message;

use super::super::response;
use super::HandleError;
use crate::server::container_client::{self, ContainerWebSocket};

/// One answer path, open: send frames, then finish.
///
/// # Finishing is something you say, not something you stop doing
///
/// [`finish`](Self::finish) is the clean close — the answer whole,
/// the command over, its items all sent, or its `Error` last. Dropping this without finishing
/// closes the socket abruptly, which the wire reads as the answer
/// DYING: a command whose outcome is unknown, reported to whoever asked as failed and never run again. So a server that is done says so.
#[must_use = "dropping the handle without finishing is the answer dying"]
pub struct ExecuteHandle {
    socket: ContainerWebSocket,
}

impl ExecuteHandle {
    pub(super) fn new(socket: ContainerWebSocket) -> Self {
        ExecuteHandle { socket }
    }

    /// Send one frame.
    pub async fn send(&mut self, frame: &response::Frame<'_>) -> Result<(), HandleError> {
        let bytes = container_client::encoded(frame).map_err(HandleError::Encode)?;
        self.socket
            .send(Message::Binary(bytes))
            .await
            .map_err(HandleError::Socket)
    }

    /// Close cleanly: the answer is whole.
    pub async fn finish(self) -> Result<(), HandleError> {
        container_client::finish(self.socket).await.map_err(HandleError::Socket)
    }
}
