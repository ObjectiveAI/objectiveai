//! Starting a loop, and answering it while it runs.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::StreamExt as _;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::channel_response::mcp;
use super::execute_stream::ExecuteStream;
use super::super::request;
use crate::client::handle::{Handle, SendError};
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
/// So this takes an [`McpProxy`] and puts a task on it. The returned
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
    mcp_proxy: Arc<P>,
) -> Result<ExecuteStream, ExecuteError>
where
    P: McpProxy + 'static,
{
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    let proxying = tokio::spawn(proxy_channel_requests(
        scope.request_receiver,
        handle.clone(),
        scope.scope,
        mcp_proxy,
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
    mut request_receiver: UnboundedReceiver<Bytes>,
    handle: Handle,
    scope: u32,
    mcp_proxy: Arc<P>,
) where
    P: McpProxy + 'static,
{
    while let Some(bytes) = request_receiver.recv().await {
        tokio::spawn(proxy_one(
            bytes,
            handle.clone(),
            scope,
            mcp_proxy.clone(),
        ));
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
/// # It stops at the first refusal
///
/// A [`Handle`] answers whether a frame went out, and `false` means no
/// later one will either — the connection is gone, or the scope is. So
/// this returns rather than going on, which matters most for the answer
/// that would otherwise never stop: an event stream held open for a
/// session, being written at a provider that is no longer listening.
///
/// It is the only thing that ends this task early. Nothing cancels it,
/// so without the check a tool call outliving its loop would run for as
/// long as its own body stream did.
async fn proxy_one<P>(
    bytes: Bytes,
    handle: Handle,
    scope: u32,
    mcp_proxy: Arc<P>,
)
where
    P: McpProxy,
{
    let Ok(frame::server::ServerFrame::ChannelRequest {
        channel, payload, ..
    }) = frame::server::ServerFrame::decode(&bytes)
    else {
        return;
    };
    // TODO: the four typed exchanges are on the wire and nothing serves
    // them. Doing so needs `McpProxy` to grow a method each, which is
    // the next step; until then they are declined the way an unreadable
    // frame is, and for the same reason.
    let Ok(channel_request::Frame::Mcp(request)) =
        channel_request::Frame::decode(payload)
    else {
        // The channel is known, the connection is fine, and nothing
        // is going to answer this. So it is ENDED rather than
        // abandoned: a provider waiting on it waits forever otherwise,
        // and a finish with no head is already what this protocol
        // means by there being no answer.
        let _ = handle.send_channel_response_finish(scope, channel).await;
        return;
    };
    let (head, body) = mcp_proxy.handle(request).await;

    let mut buffer = Vec::new();
    if mcp::Frame::Head(head)
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

    match body {
        Body::Single(body) => {
            if !send_body(&handle, scope, channel, &mut buffer, &body).await {
                return;
            }
        }
        Body::Stream(mut body) => {
            while let Some(piece) = body.next().await {
                if !send_body(&handle, scope, channel, &mut buffer, &piece)
                    .await
                {
                    return;
                }
            }
        }
    }
    // Nothing follows it, so there is nothing to do about a
    // failure here that returning would not already have done.
    let _ = handle.send_channel_response_finish(scope, channel).await;
}

/// One piece of a body, out.
///
/// The buffer is the one the head was built in, reused: a body frame is
/// a tag and a copy, and a fresh [`Vec`] per piece would reallocate its
/// way up from nothing for every one of them.
///
/// Answers whether to carry on, so a stream of them can stop at the
/// first refusal.
///
/// A [`bool`] rather than the [`SendError`] itself, because there is
/// one thing to do about every one of them here and it is stop. Which
/// of the three it was matters to a caller deciding whether the
/// connection is worth keeping, and this is not that caller.
///
/// Encoding cannot fail — a body is bytes and has nothing to get wrong
/// — so a failure there is treated as a refusal rather than handled.
/// Saying it in a match on
/// [`Infallible`](std::convert::Infallible) is not available here,
/// because the frame it shares an impl with can fail.
async fn send_body(
    handle: &Handle,
    scope: u32,
    channel: u32,
    buffer: &mut Vec<u8>,
    body: &[u8],
) -> bool {
    buffer.clear();
    if mcp::Frame::Body(body)
        .encode(&mut Writer::new(buffer))
        .is_err()
    {
        return false;
    }
    handle
        .send_channel_response(scope, channel, buffer)
        .await
        .is_ok()
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
                write!(f, "agentic loop request did not serialize: {error}")
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
