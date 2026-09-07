//! Writing the file.

use std::pin::pin;

use bytes::Bytes;
use futures_util::{SinkExt as _, Stream, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;

use super::super::{request, response};
use super::ExecuteError;
use crate::server::container_client::{self, ContainerClient};
use crate::server::messages::{MessageError, Messages};

/// Open `/write`, name the file, send its content, and hear whether
/// it landed.
///
/// The request goes out first, then every piece of `content` as one
/// message each — an empty piece is skipped, because the empty
/// message is the END of the content and is sent once, after the last
/// piece — and then the one answer is read: `Ok(())` is the file in
/// place, whole; [`Refused`](ExecuteError::Refused) is the proxy
/// saying it is not, with its reason.
///
/// # A content stream that fails abandons the write
///
/// The socket is closed WITHOUT the empty message, which the proxy
/// reads as the write abandoned: it discards what it wrote, and the
/// destination is untouched. The failure comes back as
/// [`Content`](ExecuteError::Content), the stream's own error, and
/// no answer is awaited — there is none.
pub async fn execute<S, E>(
    client: &ContainerClient,
    request: &request::Request,
    content: S,
) -> Result<(), ExecuteError<E>>
where
    S: Stream<Item = Result<Bytes, E>>,
{
    let mut socket = client.open("/write").await.map_err(ExecuteError::Open)?;
    let bytes = container_client::encoded(request).map_err(ExecuteError::Encode)?;
    socket
        .send(Message::Binary(bytes))
        .await
        .map_err(ExecuteError::Socket)?;

    let mut content = pin!(content);
    while let Some(piece) = content.next().await {
        match piece {
            Ok(bytes) => {
                if bytes.is_empty() {
                    continue;
                }
                socket
                    .send(Message::Binary(bytes))
                    .await
                    .map_err(ExecuteError::Socket)?;
            }
            Err(error) => {
                let _ = socket.close(None).await;
                return Err(ExecuteError::Content(error));
            }
        }
    }
    socket
        .send(Message::Binary(Bytes::new()))
        .await
        .map_err(ExecuteError::Socket)?;

    let mut messages = Messages::new(socket);
    let answer = match messages.next().await {
        None => return Err(ExecuteError::Unserved),
        Some(Err(MessageError::Text)) => return Err(ExecuteError::Text),
        Some(Err(MessageError::Socket(error))) => return Err(ExecuteError::Socket(error)),
        Some(Err(MessageError::Closed)) => return Err(ExecuteError::Closed),
        Some(Ok(bytes)) => bytes,
    };
    match response::Frame::decode(&answer) {
        Ok(response::Frame::Ok) => Ok(()),
        Ok(response::Frame::Error(reason)) => Err(ExecuteError::Refused(reason.to_owned())),
        Err(error) => Err(ExecuteError::Answer(error)),
    }
}
