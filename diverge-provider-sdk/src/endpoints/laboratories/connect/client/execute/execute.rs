//! Joining a laboratory.

use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::{Stream, StreamExt as _};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use super::super::super::server::channel_request as server_channel_request;
use super::super::channel_response;
use super::execute_handle::{ExecuteHandle, Write};
use super::execute_stream::ExecuteStream;
use super::super::{channel_request, request};
use crate::client::handle::{Handle, SendError};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::frame;
use crate::shared::container::write_bytes;
use crate::shared::error::Error;

/// Attach to a container somebody else is running.
///
/// # Why this one hands back two things
///
/// Because a connection is two jobs at once and neither is the other's
/// subject. The scope streams the container's filesystem for as long as
/// the connection lasts, and a connector reaches into the container on
/// channels of its own — reading files, writing them, calling the MCP
/// server, moving a file to another container.
///
/// Every other `execute` in this crate has one of those. A
/// [`watch`](crate::endpoints::volumes::watch) only listens, so a
/// stream is the whole of it; a
/// [`plugin`](crate::endpoints::mcp_plugin::run) is almost silent, so a
/// handle is. A connection is both, and folding them into one type
/// would have meant a caller that stopped reading the filetree had
/// stopped being able to read a file.
///
/// So the [`ExecuteStream`] is what the provider says, and the
/// [`ExecuteHandle`] is how a connector says anything. They are
/// independent: dropping the stream costs a view of the filesystem, and
/// dropping the handle leaves the laboratory.
///
/// A tuple rather than a type holding both, because a type holding both
/// would exist only to be taken apart.
///
/// # It answers one thing, and takes no proxy for it
///
/// A connection is the one endpoint where a caller is almost purely a
/// caller. A provider asks a connector for exactly one thing — the
/// content of a file the connector said it wanted to write — and asks
/// for nothing on its own account: the image was somebody else's
/// problem and so was deciding who may attach.
///
/// Which is why this takes no proxies, where a
/// [`plugin`](crate::endpoints::mcp_plugin::run::client::execute) takes
/// three. There is nothing for a caller to implement, because the
/// content of a write is not a service a connector provides — it is an
/// argument to the write it already asked for. So the task spawned here
/// serves content out of what
/// [`write`](ExecuteHandle::write) handed over, and a connector that
/// never writes never sees it work.
///
/// # It fails in only one way
///
/// The request either serializes or it does not, and then it either
/// goes out or it does not. Everything a provider might object to is
/// objected to afterwards, in a frame — including the runner refusing
/// the authorization, which is the ordinary way a connection does not
/// happen. See
/// [`ExecuteStreamError`](super::ExecuteStreamError).
///
/// # The disconnect is built here, not later
///
/// [`ExecuteHandle`] sends it when dropped, and a destructor is a poor
/// place to be encoding anything — so it is encoded now, through
/// [`channel_request::Frame`] rather than written as the byte it
/// happens to be. What a disconnect looks like on the wire is that
/// module's to say, and this is a caller like any other.
pub async fn execute(
    handle: &Handle,
    request: &request::Frame,
) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError> {
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;

    let mut disconnect = Vec::new();
    channel_request::Frame::Disconnect
        .encode(&mut Writer::new(&mut disconnect))
        .map_err(ExecuteError::Disconnect)?;

    let scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    let (write_sender, write_receiver) = mpsc::unbounded_channel();
    let serving = tokio::spawn(serve_writes(
        scope.request_receiver,
        write_receiver,
        handle.clone(),
        scope.scope,
    ));
    Ok((
        ExecuteStream::new(scope.response_receiver),
        ExecuteHandle::new(
            handle.clone(),
            scope.scope,
            Bytes::from(disconnect),
            write_sender,
            serving,
        ),
    ))
}

/// Answer the provider's requests for write content.
///
/// The one thing a connector is ever asked for. Ends when the request
/// receiver closes, which is the scope ending — a
/// [`Router`](crate::client::router::Router) drops everything under a
/// finished scope, that receiver with it. Nothing else stops it, except
/// the [`ExecuteHandle`] being dropped, which aborts it.
///
/// # Why it holds the writes rather than the writes holding it
///
/// The ask and the content arrive from opposite directions. A connector
/// hands over content when it asks for a write; the provider asks for
/// that content some time afterwards, on a channel of its own, naming
/// only a
/// [`write_id`](crate::shared::container::write_path::request::Request::write_id).
/// Several writes can be outstanding, and their asks all arrive on this
/// one receiver.
///
/// So something has to keep the content until the ask for it turns up,
/// and route by the id when it does. That is this, and it is a task
/// rather than a method because the queue it reads is the scope's, not
/// any one write's.
///
/// # The race that is not one
///
/// [`write`](ExecuteHandle::write) registers the content BEFORE it
/// sends the request, and a provider cannot ask for content it has not
/// been told about — so by the time an ask arrives, its content is
/// already in the queue. Draining the queue on every ask is what turns
/// "already sent" into "already here", and it is the same thing a
/// [`Router`](crate::client::router::Router) does with its own
/// registrations.
///
/// # One task per write, once its content is found
///
/// Because a write is as long as the file is, and serving one in place
/// would put every later write behind whichever is largest.
async fn serve_writes(
    mut request_receiver: UnboundedReceiver<Bytes>,
    mut write_receiver: UnboundedReceiver<Write>,
    handle: Handle,
    scope: u32,
) {
    let mut pending: HashMap<
        u32,
        Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>,
    > = HashMap::new();
    while let Some(bytes) = request_receiver.recv().await {
        while let Ok(write) = write_receiver.try_recv() {
            pending.insert(write.write_id, write.content);
        }
        let Ok(frame::server::ServerFrame::ChannelRequest {
            channel,
            payload,
            ..
        }) = frame::server::ServerFrame::decode(&bytes)
        else {
            continue;
        };
        let Ok(server_channel_request::Frame(request)) =
            server_channel_request::Frame::decode(payload)
        else {
            continue;
        };
        // An ask for a write nobody registered. The channel is left
        // unfinished: there is no content to send and no error to
        // report that would not be inventing one, and a provider that
        // asked about a write this end never made has already lost
        // track of which writes are outstanding.
        let Some(content) = pending.remove(&request.write_id) else {
            continue;
        };
        tokio::spawn(send_content(handle.clone(), scope, channel, content));
    }
}

/// Stream one write's content onto the channel that asked for it.
///
/// Bodies until the stream ends, then a finish — or, if the stream
/// yields an error, that error and then a finish. Which is what the
/// frame means by "an [`Error`], then a finish | the full content was
/// not streamed".
///
/// [`Error`]: channel_response::write_bytes::Frame::Error
///
/// # It stops at the first refusal
///
/// A [`Handle`] says whether a frame went out, and a failure means no
/// later one will either — the connection is gone, or the scope is. So
/// this returns rather than going on, which matters most for the thing
/// this sends: a file, which is the longest stream a connector ever
/// writes.
///
/// Returning that way leaves the channel unfinished, deliberately.
/// There is nothing left to finish it over.
async fn send_content(
    handle: Handle,
    scope: u32,
    channel: u32,
    mut content: Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>,
) {
    let mut buffer = Vec::new();
    while let Some(piece) = content.next().await {
        buffer.clear();
        match piece {
            Ok(bytes) => {
                if channel_response::write_bytes::Frame::Body(
                    write_bytes::response::Frame(&bytes),
                )
                .encode(&mut Writer::new(&mut buffer))
                .is_err()
                {
                    return;
                }
                if handle
                    .send_channel_response(scope, channel, &buffer)
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Err(error) => {
                // The last thing this channel carries. A failure to
                // encode it is treated the same as one to send it:
                // there is nothing else to say, and the finish below
                // says the content stopped either way.
                if channel_response::write_bytes::Frame::Error(error)
                    .encode(&mut Writer::new(&mut buffer))
                    .is_ok()
                {
                    let _ = handle
                        .send_channel_response(scope, channel, &buffer)
                        .await;
                }
                break;
            }
        }
    }
    // Nothing follows it, so there is nothing to do about a failure
    // here that returning would not already have done.
    let _ = handle.send_channel_response_finish(scope, channel).await;
}

/// A connection that never opened.
///
/// One of two ways to fail before there is anything to fail at, and
/// neither of them is a refusal — a runner saying no arrives as a frame
/// on a scope that opened to carry it. See
/// [`ExecuteStreamError`](super::ExecuteStreamError), which is the
/// connection that opened and then stopped without closing.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    ///
    /// Nothing was written, so nothing is waiting to answer it. See
    /// [`SendError`] for the three reasons, only one of which is about
    /// this exchange rather than the whole connection.
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The disconnect would not serialize.
    ///
    /// Which cannot happen — a disconnect is one tag byte — and is
    /// reported rather than unwrapped because the encode it shares an
    /// impl with can fail. It is built here, before the request goes
    /// out, because the only place it is used is a destructor: too late
    /// to serialize, and with nobody to tell if it went wrong.
    Disconnect(serde_json::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => {
                write!(f, "the request never went out: {error}")
            }
            ExecuteError::Request(error) => {
                write!(f, "connection request did not serialize: {error}")
            }
            ExecuteError::Disconnect(error) => {
                write!(f, "connection disconnect did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Disconnect(error) => Some(error),
        }
    }
}
