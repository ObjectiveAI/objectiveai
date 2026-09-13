//! Opening the scope, streaming the content, and hearing the answer.

use std::pin::pin;

use bytes::Bytes;
use futures_util::future::{self, Either};
use futures_util::{Stream, StreamExt as _};

use super::super::channel_response::content;
use super::super::request;
use super::super::super::server::response;
use super::ExecuteError;
use crate::CHUNK_SIZE;
use crate::client::handle::Handle;
use crate::client::scope::Scope;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::frame;
use crate::shared::containers::write_bytes;
use crate::shared::error::Error;

/// Write `content` to the file at `path`, as components from the
/// container's root, and wait for the proxy to say it landed.
///
/// The content travels on a channel the PROXY opens — only a
/// responder can finish a channel — so this waits for that channel,
/// answers it with the content in pieces of at most
/// [`CHUNK_SIZE`], finishes it, and then reads the scope's answer:
/// `Written` is [`Ok`]; an error is [`Refused`](ExecuteError::Refused),
/// the proxy's own words. A proxy that answers before asking for the
/// content — a missing parent, a destination it cannot replace — is
/// read the same way, and the content is not sent.
///
/// A second channel the proxy opens on the scope is one this end
/// cannot serve, and is finished with nothing before the finish.
///
/// A piece of `content` that is an error is the write abandoned: the
/// error goes to the proxy as the content channel's own, the channel
/// finishes, and this returns [`Content`](ExecuteError::Content)
/// without waiting for the proxy, which discards what it wrote.
pub async fn execute<S>(handle: &Handle, path: Vec<String>, content: S) -> Result<(), ExecuteError>
where
    S: Stream<Item = Result<Bytes, Error>>,
{
    let mut payload = Vec::new();
    request::Frame { path }
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let mut scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;

    // The proxy's ask for the content, or its answer first.
    let channel = match content_channel(&mut scope).await? {
        Opened::Channel(channel) => channel,
        Opened::Answered(answer) => return answer,
    };

    let mut content = pin!(content);
    while let Some(piece) = content.next().await {
        match piece {
            Ok(bytes) => {
                for piece in bytes.chunks(CHUNK_SIZE) {
                    let frame = content::Frame::Body(write_bytes::response::Frame(piece));
                    let mut payload = Vec::new();
                    frame.encode(&mut Writer::new(&mut payload)).map_err(ExecuteError::Request)?;
                    handle
                        .send_channel_response(scope.scope, channel, &payload)
                        .await
                        .map_err(ExecuteError::Send)?;
                }
            }
            Err(error) => {
                let frame = content::Frame::Error(error.clone());
                let mut payload = Vec::new();
                if frame.encode(&mut Writer::new(&mut payload)).is_ok() {
                    let _ = handle.send_channel_response(scope.scope, channel, &payload).await;
                }
                let _ = handle.send_channel_response_finish(scope.scope, channel).await;
                return Err(ExecuteError::Content(error));
            }
        }
    }
    handle
        .send_channel_response_finish(scope.scope, channel)
        .await
        .map_err(ExecuteError::Send)?;

    // A second channel the proxy opened on this scope is one this end
    // cannot serve: the finish with nothing before it.
    while let Ok(bytes) = scope.request_receiver.try_recv() {
        if let Ok(frame::server::ServerFrame::ChannelRequest { channel: stray, .. }) =
            frame::server::ServerFrame::decode(&bytes)
        {
            let _ = handle.send_channel_response_finish(scope.scope, stray).await;
        }
    }

    let bytes = scope.response_receiver.recv().await.ok_or(ExecuteError::Closed)?;
    answer(&bytes)
}

/// What arrived first on a write's scope.
enum Opened {
    /// The proxy asked for the content, on this channel of its own.
    Channel(u32),
    /// The proxy answered the write before asking for anything.
    Answered(Result<(), ExecuteError>),
}

/// Wait for the proxy's content channel, or for its answer.
async fn content_channel(scope: &mut Scope) -> Result<Opened, ExecuteError> {
    loop {
        let request = pin!(scope.request_receiver.recv());
        let response = pin!(scope.response_receiver.recv());
        match future::select(request, response).await {
            Either::Left((Some(bytes), _)) => {
                if let Ok(frame::server::ServerFrame::ChannelRequest { channel, .. }) =
                    frame::server::ServerFrame::decode(&bytes)
                {
                    // The ask carries nothing: the scope is the write.
                    return Ok(Opened::Channel(channel));
                }
            }
            Either::Left((None, _)) => return Err(ExecuteError::Closed),
            Either::Right((Some(bytes), _)) => return Ok(Opened::Answered(answer(&bytes))),
            Either::Right((None, _)) => return Err(ExecuteError::Closed),
        }
    }
}

/// The scope's answer: the file landed, or why not.
fn answer(bytes: &[u8]) -> Result<(), ExecuteError> {
    let envelope = frame::server::ServerFrame::decode(bytes).map_err(ExecuteError::Frame)?;
    let payload = match envelope {
        frame::server::ServerFrame::Response { payload, .. } => payload,
        frame::server::ServerFrame::ResponseFinish { .. } => return Err(ExecuteError::Unanswered),
        _ => return Err(ExecuteError::Misrouted),
    };
    match response::Frame::decode(payload).map_err(ExecuteError::Response)? {
        response::Frame::Written(_) => Ok(()),
        response::Frame::Error(error) => Err(ExecuteError::Refused(error)),
    }
}
