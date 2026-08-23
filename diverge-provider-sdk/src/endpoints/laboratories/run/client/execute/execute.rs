//! Running a laboratory, and answering what it needs while it runs.

use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::{Stream, StreamExt as _};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use super::super::request;
use super::super::super::server::channel_request as server_channel_request;
use super::super::channel_response;
use super::execute_handle::{ExecuteHandle, Write};
use super::execute_stream::ExecuteStream;
use crate::client::handle::{Handle, SendError};
use crate::client::laboratory_connection_authorizer::{
    Decision, LaboratoryConnectionAuthorizer,
};
use crate::client::oci_proxy::OciProxy;
use crate::shared::oci;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::frame;
use crate::shared::container::write_bytes;
use crate::shared::error::Error;

/// Run a container, and answer what it needs for as long as it lasts.
///
/// # Two things back, and two proxies in
///
/// The [`ExecuteStream`] is what the container reports — its id, its
/// filesystem, and connectors leaving. The [`ExecuteHandle`] is how a
/// runner reaches into it and how a runner stops it. They are split
/// because a runner that has stopped watching has not necessarily
/// stopped working, and one that wants to stop the container should not
/// have to hold a stream to do it.
///
/// What goes in is an [`OciProxy`], for an image the provider cannot
/// pull itself, and a [`LaboratoryConnectionAuthorizer`], for the
/// connectors that turn up. Both are optional in practice and neither
/// is optional in the signature: what a run will be asked for is not
/// knowable when it is started.
///
/// # It is the only endpoint that is asked all three ways
///
/// A [`plugin`](crate::endpoints::mcp_plugin::run::client::execute) is
/// asked for three things and answers each with a proxy. A
/// [`connection`](crate::endpoints::laboratories::connect::client::execute)
/// is asked for one thing and answers it with an argument. A run is
/// asked for all of it — an image through a proxy, an authorization
/// through a proxy, and the content of a write through the
/// [`write`](ExecuteHandle::write) that asked for it.
///
/// Which is why the task spawned here is the two dispatchers this crate
/// already had, merged: a branch per kind of ask, and a registry for
/// the one kind whose answer was handed over before the ask arrived.
///
/// # An authorization holds somebody up
///
/// Alone among the things a caller answers. A connector is blocked
/// until the authorizer decides, so an implementation that is slow
/// makes arrivals slow — see
/// [`LaboratoryConnectionAuthorizer`] for the whole of that.
///
/// Each one is answered on its own task, so a slow decision delays the
/// connector it is about and nothing else.
///
/// # It fails in only one way
///
/// The request either serializes or it does not, and then it either
/// goes out or it does not. Everything a provider might object to is
/// objected to afterwards, in a frame. See
/// [`ExecuteStreamError`](super::ExecuteStreamError).
pub async fn execute<O, A>(
    handle: &Handle,
    request: &request::Frame,
    oci_proxy: Arc<O>,
    authorizer: Arc<A>,
) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError>
where
    O: OciProxy + 'static,
    A: LaboratoryConnectionAuthorizer + 'static,
{
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    let (write_sender, write_receiver) = mpsc::unbounded_channel();
    tokio::spawn(serve(
        scope.request_receiver,
        write_receiver,
        handle.clone(),
        scope.scope,
        oci_proxy,
        authorizer,
    ));
    Ok((
        ExecuteStream::new(scope.response_receiver),
        ExecuteHandle::new(handle.clone(), scope.scope, write_sender),
    ))
}

/// Answer everything the provider asks for.
///
/// Ends when the request receiver closes, which is the scope ending — a
/// [`Router`](crate::client::router::Router) drops everything under a
/// finished scope, that receiver with it. Nothing else stops it; the
/// task is detached, so it outlives a runner that has stopped caring
/// and ends with the scope regardless.
///
/// # Why one task and not three
///
/// Because the three kinds of ask arrive on one queue. The frame layer
/// sorts by scope, not by what is inside a payload, so something has to
/// read every channel request and decide which of the three it is.
///
/// Having decided, it spawns. An image layer is hundreds of megabytes,
/// a file is as long as it is, and an authorization is as slow as
/// whoever is deciding — serving any of them in place would put every
/// later ask behind it.
///
/// # The write registry, and the race that is not one
///
/// A write is the odd one: its content was handed over by
/// [`write`](ExecuteHandle::write) BEFORE the provider asked for it, so
/// something must keep it until the ask turns up and route by
/// [`write_id`](crate::shared::container::write_path::request::Request::write_id).
///
/// The registration goes out before the write request does, and a
/// provider cannot ask for content it has not been told about — so by
/// the time an ask arrives, its content is already in the queue.
/// Draining on every ask is what turns "already sent" into "already
/// here", the same way a
/// [`Router`](crate::client::router::Router) treats its own
/// registrations.
async fn serve<O, A>(
    mut request_receiver: UnboundedReceiver<Bytes>,
    mut write_receiver: UnboundedReceiver<Write>,
    handle: Handle,
    scope: u32,
    oci_proxy: Arc<O>,
    authorizer: Arc<A>,
) where
    O: OciProxy + 'static,
    A: LaboratoryConnectionAuthorizer + 'static,
{
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
        match server_channel_request::Frame::decode(payload) {
            Ok(server_channel_request::Frame::Oci(request)) => {
                // Decoded again inside the task: a request borrows the
                // frame it came out of, and a borrow cannot cross a
                // spawn.
                tokio::spawn(serve_oci(
                    bytes.clone(),
                    handle.clone(),
                    scope,
                    Arc::clone(&oci_proxy),
                ));
                let _ = request;
            }
            Ok(server_channel_request::Frame::Authorize(request)) => {
                tokio::spawn(serve_authorize(
                    request,
                    handle.clone(),
                    scope,
                    channel,
                    Arc::clone(&authorizer),
                ));
            }
            Ok(server_channel_request::Frame::Write(request)) => {
                // An ask for a write nobody registered. The channel is
                // left unfinished: there is no content to send and no
                // error to report that would not be inventing one, and
                // a provider that asked about a write this end never
                // made has already lost track of which writes are
                // outstanding.
                let Some(content) = pending.remove(&request.write_id) else {
                    continue;
                };
                tokio::spawn(send_content(
                    handle.clone(),
                    scope,
                    channel,
                    content,
                ));
            }
            Err(_) => {}
        }
    }
}

/// Answer one registry request.
///
/// The bytes the proxy produces go back as they arrive, as one frame or
/// as many, and the channel finishes after. There is no head: the answer
/// IS the registry's, headers and status and body together, and this end
/// carries it rather than describing it — see
/// [`oci`](crate::shared::oci) for why the exchange is bytes.
///
/// # It decodes the frame again
///
/// Because the request borrows the bytes it was decoded from, and a
/// borrow cannot cross a spawn. The [`Bytes`] is refcounted, so what
/// crosses is a pointer and the second decode is a parse of memory that
/// was already there.
///
/// # It stops at the first refusal
///
/// A [`Handle`] says whether a frame went out, and a failure means no
/// later one will either. So this returns rather than going on, which
/// matters most for a layer: a blob is the longest thing this protocol
/// sends, and going on writing one at a provider that has stopped
/// listening is the most work available to waste.
async fn serve_oci<O>(bytes: Bytes, handle: Handle, scope: u32, oci_proxy: Arc<O>)
where
    O: OciProxy,
{
    let Ok(frame::server::ServerFrame::ChannelRequest {
        channel, payload, ..
    }) = frame::server::ServerFrame::decode(&bytes)
    else {
        return;
    };
    let Ok(server_channel_request::Frame::Oci(request)) =
        server_channel_request::Frame::decode(payload)
    else {
        // The channel is known, the connection is fine, and nothing
        // is going to answer this. So it is ENDED rather than
        // abandoned: a provider waiting on it waits forever otherwise,
        // and a finish with no head is already what this protocol
        // means by there being no answer.
        let _ = handle.send_channel_response_finish(scope, channel).await;
        return;
    };
    // Refcounted rather than copied: the answer outlives this frame by
    // as long as a layer takes to send, so a borrow could not have gone
    // with it.
    let request = bytes.slice_ref(request.0);

    let mut answer = oci_proxy.handle(request).await;
    let mut buffer = Vec::new();
    while let Some(piece) = answer.next().await {
        if !send_oci_body(&handle, scope, channel, &mut buffer, &piece).await {
            return;
        }
    }
    // Nothing follows it, so there is nothing to do about a failure
    // here that returning would not already have done.
    let _ = handle.send_channel_response_finish(scope, channel).await;
}

/// One piece of a registry answer, out.
///
/// The buffer is reused across pieces: a frame is a copy, and a fresh
/// [`Vec`] per piece would reallocate its way up from nothing for every
/// one of them.
///
/// Answers whether to carry on, so a stream of them can stop at the
/// first refusal. Encoding cannot fail — an answer is bytes and has
/// nothing to get wrong — so the only `false` is a send that did not
/// land.
async fn send_oci_body(
    handle: &Handle,
    scope: u32,
    channel: u32,
    buffer: &mut Vec<u8>,
    body: &[u8],
) -> bool {
    buffer.clear();
    // The shared type rather than this endpoint's alias of it: an alias
    // names a tuple struct but cannot construct one.
    oci::response::Frame(body)
        .encode(&mut Writer::new(buffer))
        .unwrap_or_else(|error| match error {});
    handle
        .send_channel_response(scope, channel, buffer)
        .await
        .is_ok()
}

/// Decide one connector, and say so.
///
/// One frame and then a finish, which is the whole of this exchange —
/// an authorization is not a stream and the channel closes as soon as
/// the answer is out.
///
/// # A connector is waiting on this
///
/// The provider does not let the connection open until the answer
/// arrives, so the time this takes is time somebody spends waiting at a
/// door. It is on its own task for that reason: a slow decision delays
/// the connector it is about and nothing else.
///
/// # A failure to answer is not a denial
///
/// If the frame cannot go out, this says nothing at all — and a
/// provider waiting on an answer that never comes is a connector that
/// never attaches, which looks like a denial from the outside and is
/// not one. There is nothing better available: the channel that would
/// have carried "no" is the channel that is not working.
async fn serve_authorize<A>(
    request: server_channel_request::Authorize,
    handle: Handle,
    scope: u32,
    channel: u32,
    authorizer: Arc<A>,
) where
    A: LaboratoryConnectionAuthorizer,
{
    let decision = authorizer.handle(request).await;
    let frame = match &decision {
        Decision::Denied => channel_response::authorize::Frame::Denied,
        Decision::Authorized(nickname) => {
            channel_response::authorize::Frame::Authorized(nickname)
        }
    };
    let mut payload = Vec::new();
    frame
        .encode(&mut Writer::new(&mut payload))
        .unwrap_or_else(|error| match error {});
    if handle
        .send_channel_response(scope, channel, &payload)
        .await
        .is_err()
    {
        return;
    }
    let _ = handle.send_channel_response_finish(scope, channel).await;
}

/// Stream one write's content onto the channel that asked for it.
///
/// Bodies until the stream ends, then a finish — or, if the stream
/// yields an error, that error and then a finish. Which is what the
/// frame means by "an `Error`, then a finish | the full content was not
/// streamed".
///
/// # It stops at the first refusal
///
/// Returning that way leaves the channel unfinished, deliberately.
/// There is nothing left to finish it over.
///
/// A body that will not ENCODE is the other case and not that one:
/// nothing went out, so the connection is still working and the channel
/// is still owed its finish. That one stops the content and finishes
/// anyway.
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
                // Breaking rather than returning: an encode that failed
                // sent nothing, so the connection is still working and
                // the channel is still owed its finish. Only a send
                // that did not land means there is nobody to finish it
                // at.
                if channel_response::write_bytes::Frame::Body(
                    write_bytes::response::Frame(&bytes),
                )
                .encode(&mut Writer::new(&mut buffer))
                .is_err()
                {
                    break;
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

/// A laboratory that never started.
///
/// Two ways to fail before there is anything to fail at, and neither of
/// them is a refusal — a provider that will not run the container says
/// so in a frame, on a scope that opened to carry it. See
/// [`ExecuteStreamError`](super::ExecuteStreamError), which is the
/// laboratory that started and then stopped without ending.
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
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => {
                write!(f, "the request never went out: {error}")
            }
            ExecuteError::Request(error) => {
                write!(f, "laboratory request did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
        }
    }
}
