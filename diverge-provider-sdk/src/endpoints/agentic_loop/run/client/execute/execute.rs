//! Starting a loop, and answering it while it runs.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

use super::execute_stream::ExecuteStream;
use super::super::request;
use crate::client::handle::{Handle, SendError};
use crate::client::mcp_proxy::McpProxy;
use crate::encode::{Encode, Writer};
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

/// Answer one thing the agent asked for.
///
/// # It answers none of them yet
///
/// The five exchanges an agent can open are on the wire and nothing
/// serves them. Serving them needs
/// [`McpProxy`](crate::client::mcp_proxy::McpProxy) to grow a method
/// each — the tunneled HTTP request it answers today is gone from this
/// endpoint — so every channel is declined until it has them.
///
/// Declined, and not abandoned. The channel is finished with nothing
/// before it, which this protocol already means as there being no
/// answer: a provider waiting on one waits forever otherwise, and
/// nothing anywhere would time it out.
async fn proxy_one<P>(
    bytes: Bytes,
    handle: Handle,
    scope: u32,
    mcp_proxy: Arc<P>,
)
where
    P: McpProxy,
{
    // TODO: nothing serves the five typed exchanges. Serving them needs
    // `McpProxy` to grow a method each — the tunneled HTTP request it
    // answers today is gone from the wire — and until it has them there
    // is nothing to hand a request to.
    //
    // The proxy is still threaded here rather than removed, because it
    // is what will answer these and taking it out would churn this
    // module's signature twice.
    let _ = mcp_proxy;

    let Ok(frame::server::ServerFrame::ChannelRequest { channel, .. }) =
        frame::server::ServerFrame::decode(&bytes)
    else {
        return;
    };

    // The channel is known, the connection is fine, and nothing is
    // going to answer this. So it is ENDED rather than abandoned: a
    // provider waiting on it waits forever otherwise, and a finish with
    // nothing before it is already what this protocol means by there
    // being no answer.
    let _ = handle.send_channel_response_finish(scope, channel).await;
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
