//! Starting a loop, and answering it while it runs.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::StreamExt as _;
use tokio::sync::mpsc::UnboundedReceiver;

use super::channel_response::mcp;
use super::execute_stream::ExecuteStream;
use super::request;
use crate::client::handle::Handle;
use crate::client::mcp_proxy::{Body, McpProxy};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::agentic_loop::run::server::channel_request;
use crate::frame;

/// Run an agent, and answer its tools while it does.
///
/// # Two things at once, which is what makes this one different
///
/// Every other `execute` in this crate asks and then listens. A loop
/// asks, listens, and is asked back: an agent's tool calls arrive as
/// MCP requests on channels the provider opens, because the MCP servers
/// live with the caller and the agent runs beside the provider.
///
/// So this takes a [`McpProxy`] and puts a task on it. The returned
/// [`ExecuteStream`] carries chunks and nothing else; the answering
/// happens beside it and neither waits on the other. A caller that
/// stops to think about a chunk would otherwise be a caller that has
/// stopped answering the agent's tools, and an agent waiting on a tool
/// produces no chunks — which is a stall that feeds itself.
///
/// # One task per exchange, not one for all of them
///
/// The loop reading channel requests spawns rather than serving in
/// place, because an MCP answer can be an event stream held open for a
/// session. Serving in place would put every later tool call behind
/// whichever one is longest-lived, which for a session stream means
/// behind one that never ends.
///
/// # An [`Arc`] rather than a value
///
/// So one proxy can serve several loops. It is what a caller has
/// anyway: an MCP proxy is a thing you stand up once, and a run is a
/// thing you do many times.
///
/// # It fails in only one way
///
/// The request either serializes or it does not. Everything after that
/// belongs to the loop rather than to the asking — a provider that
/// cannot run the agent says so in a frame like everything else. See
/// [`ExecuteStreamError`](super::ExecuteStreamError).
///
/// # There is no depth to choose
///
/// Both of the scope's queues are unbounded, so neither the agent's
/// chunks nor its tool calls can stall on a slow reader. The tool calls
/// would not have anyway — the task taking them off does nothing but
/// spawn — but the chunks could, and a caller that stalled its own
/// chunk stream would have been stalling every other scope on the
/// connection with it.
///
/// What it costs is a bound. A caller that stops reading grows a queue
/// instead of stopping anything, and dropping the [`ExecuteStream`] is
/// what frees it.
pub async fn execute<P>(
    handle: &Handle,
    request: &request::Frame,
    proxy: Arc<P>,
) -> Result<ExecuteStream, ExecuteError>
where
    P: McpProxy + 'static,
{
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await;
    let proxying = tokio::spawn(proxy_channel_requests(
        scope.request_receiver,
        handle.clone(),
        scope.scope,
        proxy,
    ));
    Ok(ExecuteStream::new(scope.response_receiver, proxying))
}

/// Take the agent's tool calls off the queue, and put each on its own
/// task.
///
/// Ends when the receiver closes, which is the scope ending — a
/// [`Router`](crate::client::router::Router) drops everything under a
/// finished scope, this receiver with it. Nothing else stops it, except
/// the [`ExecuteStream`] being dropped, which aborts it.
///
/// The frame is passed on whole and undecoded. Decoding it here would
/// mean either borrowing across the spawn, which cannot be done, or
/// copying out of it, which would be a copy per tool call for nothing.
async fn proxy_channel_requests<P>(
    mut requests: UnboundedReceiver<Bytes>,
    handle: Handle,
    scope: u32,
    proxy: Arc<P>,
) where
    P: McpProxy + 'static,
{
    while let Some(bytes) = requests.recv().await {
        tokio::spawn(proxy_one(bytes, handle.clone(), scope, proxy.clone()));
    }
}

/// Answer one tool call.
///
/// The head goes back first and then the body, as one frame or as many,
/// and the channel finishes after. That order is the wire's — see
/// [`http::response::Frame`](crate::shared::http::response::Frame) for
/// why a head arrives separately from the body it introduces.
///
/// # What a frame it cannot read does
///
/// Nothing, and the channel is left unfinished. There is no error
/// variant on an MCP channel — the exchange is HTTP and HTTP says how
/// things go wrong — but a request this end cannot even decode has no
/// status to answer with, because it is not an HTTP request as far as
/// this can tell. Finishing an unanswered channel would say the answer
/// was empty, which is a different and worse thing to say.
///
/// A head that will not serialize is the same case one step later.
///
/// # It never sees the scope end
///
/// Its writes simply stop landing. A channel response for a scope that
/// has closed is a frame the provider discards, and nothing here checks
/// first — the check would be stale by the time it was acted on, and
/// the frame is harmless.
async fn proxy_one<P>(bytes: Bytes, handle: Handle, scope: u32, proxy: Arc<P>)
where
    P: McpProxy,
{
    let Ok(frame::server::ServerFrame::ChannelRequest {
        channel, payload, ..
    }) = frame::server::ServerFrame::decode(&bytes)
    else {
        return;
    };
    let Ok(channel_request::Frame(request)) =
        channel_request::Frame::decode(payload)
    else {
        return;
    };
    let (head, body) = proxy.forward(request).await;

    let mut buffer = Vec::new();
    if mcp::Frame::Head(head)
        .encode(&mut Writer::new(&mut buffer))
        .is_err()
    {
        return;
    }
    handle.send_channel_response(scope, channel, &buffer).await;

    match body {
        Body::Single(body) => {
            send_body(&handle, scope, channel, &mut buffer, &body).await;
        }
        Body::Stream(mut body) => {
            while let Some(piece) = body.next().await {
                send_body(&handle, scope, channel, &mut buffer, &piece).await;
            }
        }
    }
    handle.send_channel_response_finish(scope, channel).await;
}

/// One piece of a body, out.
///
/// The buffer is the one the head was built in, reused: a body frame is
/// a tag and a copy, and a fresh [`Vec`] per piece would reallocate its
/// way up from nothing for every one of them.
///
/// Encoding cannot fail — a body is bytes and has nothing to get wrong
/// — so the [`Result`] is discarded rather than handled. Saying that in
/// a match on
/// [`Infallible`](std::convert::Infallible) is not available here,
/// because the frame it shares an impl with can.
async fn send_body(
    handle: &Handle,
    scope: u32,
    channel: u32,
    buffer: &mut Vec<u8>,
    body: &[u8],
) {
    buffer.clear();
    if mcp::Frame::Body(body)
        .encode(&mut Writer::new(buffer))
        .is_err()
    {
        return;
    }
    handle.send_channel_response(scope, channel, buffer).await;
}

/// A loop that never started.
///
/// One way, because starting one is only serializing the request and
/// writing it. Everything a provider might object to is objected to
/// afterwards, in a frame — see
/// [`ExecuteStreamError`](super::ExecuteStreamError), which is the loop
/// that started and then stopped without ending.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request would not serialize.
    Request(serde_json::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Request(error) => {
                write!(f, "agentic loop request did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Request(error) => Some(error),
        }
    }
}
