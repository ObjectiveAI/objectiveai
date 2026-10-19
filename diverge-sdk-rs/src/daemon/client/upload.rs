//! Sending files on the channels the daemon opens, and hearing the
//! answer.

use std::fmt;
use std::pin::pin;

use bytes::Bytes;
use futures_util::future::{self, Either};
use futures_util::{Stream, StreamExt as _};

use super::one_shot;
use crate::CHUNK_SIZE;
use crate::provider::endpoints::volumes::write::client::channel_response;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;
use crate::shared::containers::write_bytes;
use crate::shared::error::Error;

/// How a content ask is read: the path it names, or `None` for the one
/// file of a file upload, from the channel request's payload; a
/// payload that will not decode is `Err`.
pub type AskDecoder = fn(&[u8]) -> Result<Option<String>, serde_json::Error>;

/// Send an upload request, answer every content channel the daemon
/// opens on its scope from `source`, and read the one answer.
///
/// The content travels on channels the DAEMON opens — only a responder
/// can finish a channel — one per file. Each ask names the file, as the
/// request named it, or nothing for a file upload; `source` is asked
/// for that file's content and hands back a stream of pieces, which go
/// out as the channel's responses in pieces of at most [`CHUNK_SIZE`],
/// and the channel finishes. A file `source` has no content for, or a
/// piece that is an error, sends the channel's own error and finishes
/// it; the daemon abandons that file and answers the scope with its
/// error. The first frame on the scope's response receiver is the
/// answer, whichever the daemon chose, and this returns it; a daemon
/// that answers before asking for anything — a container it cannot
/// find, forbidden — is read the same way, and no content is sent.
///
/// Content channels are answered on tasks of their own, so files
/// stream at once; one still streaming when the answer arrives is
/// abandoned with the scope.
pub async fn execute<Q, A, E, S, F>(
    handle: &Handle,
    request: &Q,
    decode_ask: AskDecoder,
    mut source: F,
) -> Result<A, UploadError<Q::Error, E>>
where
    Q: Encode,
    A: for<'a> Decode<'a, Error = E>,
    S: Stream<Item = Result<Bytes, Error>> + Send + 'static,
    F: FnMut(Option<String>) -> Option<S>,
{
    let mut scope = one_shot::open::<Q, E>(handle, request).await.map_err(UploadError::from)?;
    loop {
        let request = pin!(scope.request_receiver.recv());
        let response = pin!(scope.response_receiver.recv());
        match future::select(request, response).await {
            Either::Left((Some(bytes), _)) => {
                let Ok(frame::server::ServerFrame::ChannelRequest { channel, payload, .. }) = frame::server::ServerFrame::decode(&bytes) else {
                    continue;
                };
                let content = match decode_ask(payload) {
                    Ok(path) => source(path),
                    Err(_) => None,
                };
                tokio::spawn(answer(handle.clone(), scope.scope, channel, content));
            }
            Either::Left((None, _)) => return Err(UploadError::Closed),
            Either::Right((Some(bytes), _)) => return one_shot::answer(&bytes).map_err(UploadError::from),
            Either::Right((None, _)) => return Err(UploadError::Closed),
        }
    }
}

/// One content channel answered: the pieces, then the finish; or the
/// error, then the finish.
async fn answer<S>(handle: Handle, scope: u32, channel: u32, content: Option<S>)
where
    S: Stream<Item = Result<Bytes, Error>>,
{
    let Some(content) = content else {
        refuse(&handle, scope, channel, Error(serde_json::Value::String("no content for the file asked for".to_string()))).await;
        return;
    };
    let mut content = pin!(content);
    while let Some(piece) = content.next().await {
        match piece {
            Ok(bytes) => {
                for piece in bytes.chunks(CHUNK_SIZE) {
                    let frame = channel_response::Frame::Body(write_bytes::response::Frame(piece));
                    let mut payload = Vec::new();
                    if frame.encode(&mut Writer::new(&mut payload)).is_err() {
                        break;
                    }
                    if handle.send_channel_response(scope, channel, &payload).await.is_err() {
                        return;
                    }
                }
            }
            Err(error) => {
                refuse(&handle, scope, channel, error).await;
                return;
            }
        }
    }
    let _ = handle.send_channel_response_finish(scope, channel).await;
}

/// The channel's own error, then its finish: the daemon abandons the
/// file.
async fn refuse(handle: &Handle, scope: u32, channel: u32, error: Error) {
    let frame = channel_response::Frame::Error(error);
    let mut payload = Vec::new();
    if frame.encode(&mut Writer::new(&mut payload)).is_ok() {
        let _ = handle.send_channel_response(scope, channel, &payload).await;
    }
    let _ = handle.send_channel_response_finish(scope, channel).await;
}

/// An upload that did not produce an answer.
#[derive(Debug)]
pub enum UploadError<Q, A> {
    /// The request never went out. See [`SendError`].
    Send(SendError),
    /// The request did not serialize.
    Request(Q),
    /// The connection ended before an answer arrived.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The daemon finished the scope without answering.
    Unanswered,
    /// A frame that cannot be the first on a main stream.
    Misrouted,
    /// The answer did not parse.
    Response(A),
}

impl<Q, A> From<one_shot::Error<Q, A>> for UploadError<Q, A> {
    fn from(error: one_shot::Error<Q, A>) -> Self {
        match error {
            one_shot::Error::Send(error) => UploadError::Send(error),
            one_shot::Error::Request(error) => UploadError::Request(error),
            one_shot::Error::Closed => UploadError::Closed,
            one_shot::Error::Frame(error) => UploadError::Frame(error),
            one_shot::Error::Unanswered => UploadError::Unanswered,
            one_shot::Error::Misrouted => UploadError::Misrouted,
            one_shot::Error::Response(error) => UploadError::Response(error),
        }
    }
}

impl<Q: fmt::Display, A: fmt::Display> fmt::Display for UploadError<Q, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UploadError::Send(error) => write!(f, "the upload request never went out: {error}"),
            UploadError::Request(error) => write!(f, "the upload request did not serialize: {error}"),
            UploadError::Closed => f.write_str("the connection ended before the upload was answered"),
            UploadError::Frame(error) => write!(f, "the answer did not decode: {error}"),
            UploadError::Unanswered => f.write_str("the daemon finished the upload without an answer"),
            UploadError::Misrouted => f.write_str("a frame that cannot open an answer arrived"),
            UploadError::Response(error) => write!(f, "the upload answer did not parse: {error}"),
        }
    }
}

impl<Q, A> std::error::Error for UploadError<Q, A>
where
    Q: std::error::Error + 'static,
    A: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UploadError::Send(error) => Some(error),
            UploadError::Request(error) => Some(error),
            UploadError::Frame(error) => Some(error),
            UploadError::Response(error) => Some(error),
            UploadError::Closed | UploadError::Unanswered | UploadError::Misrouted => None,
        }
    }
}
