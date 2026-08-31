//! Starting a loop, and answering it while it runs.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

use futures_util::StreamExt as _;

use super::execute_stream::ExecuteStream;
use super::super::channel_response::{
    fetch_directory, fetch_file, fetch_resource, mcp_call_tool,
    mcp_list_resources, mcp_list_tools, mcp_notifications,
    mcp_read_resource,
};
use super::super::request;
use crate::client::fetch_proxy::FetchProxy;
use crate::client::handle::{Handle, SendError};
use crate::decode::Decode;
use crate::endpoints::agentic_loop::run::server::channel_request;
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
/// It takes a [`FetchProxy`] for the same reason: a provider missing
/// the content behind a dirhash asks for it on a channel of its own,
/// and a run that cannot be furnished is a run that never starts.
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
pub async fn execute<P, F>(
    handle: &Handle,
    request: &request::Frame,
    mcp_proxy: Arc<P>,
    fetch_proxy: Arc<F>,
) -> Result<ExecuteStream, ExecuteError>
where
    P: McpProxy + 'static,
    F: FetchProxy + 'static,
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
        fetch_proxy,
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
async fn proxy_channel_requests<P, F>(
    mut request_receiver: UnboundedReceiver<Bytes>,
    handle: Handle,
    scope: u32,
    mcp_proxy: Arc<P>,
    fetch_proxy: Arc<F>,
) where
    P: McpProxy + 'static,
    F: FetchProxy + 'static,
{
    while let Some(bytes) = request_receiver.recv().await {
        tokio::spawn(proxy_one(
            bytes,
            handle.clone(),
            scope,
            mcp_proxy.clone(),
            fetch_proxy.clone(),
        ));
    }
}

/// Answer one thing the server asked for.
///
/// Six things it can be, and each is answered once — except the
/// notification stream, which is answered until it stops, and the
/// fetch, which is answered once per file.
///
/// # A frame it cannot read is ENDED, not abandoned
///
/// The channel is known and the connection is fine; only the payload is
/// unreadable, which is what a provider newer than this client
/// produces. Finishing it with nothing before it is what this protocol
/// already means by there being no answer — where returning would leave
/// a provider waiting forever, since nothing anywhere times one out.
async fn proxy_one<P, F>(
    bytes: Bytes,
    handle: Handle,
    scope: u32,
    mcp_proxy: Arc<P>,
    fetch_proxy: Arc<F>,
) where
    P: McpProxy,
    F: FetchProxy,
{
    let Ok(frame::server::ServerFrame::ChannelRequest {
        channel, payload, ..
    }) = frame::server::ServerFrame::decode(&bytes)
    else {
        return;
    };

    let Ok(request) = channel_request::Frame::decode(payload) else {
        let _ = handle.send_channel_response_finish(scope, channel).await;
        return;
    };

    // Whatever happened, the answer is over. Ignoring the outcome is
    // deliberate: an answer that could not be ENCODED leaves a healthy
    // connection with an unfinished channel on it, and a provider
    // waiting on one waits forever. An answer that could not be SENT
    // means the connection is gone and this fails too, harmlessly.
    match request {
        channel_request::Frame::McpListTools(request) => {
            let answer = match mcp_proxy.list_tools(request.0).await {
                Ok(result) => mcp_list_tools::Frame::Result(result),
                Err(error) => mcp_list_tools::Frame::Error(error),
            };
            answer_with(&handle, scope, channel, &answer).await
        }
        channel_request::Frame::McpListResources(request) => {
            let answer = match mcp_proxy.list_resources(request.0).await {
                Ok(result) => mcp_list_resources::Frame::Result(result),
                Err(error) => mcp_list_resources::Frame::Error(error),
            };
            answer_with(&handle, scope, channel, &answer).await
        }
        channel_request::Frame::McpCallTool(request) => {
            let answer = match mcp_proxy.call_tool(request.0).await {
                Ok(result) => mcp_call_tool::Frame::Result(result),
                Err(error) => mcp_call_tool::Frame::Error(error),
            };
            answer_with(&handle, scope, channel, &answer).await
        }
        channel_request::Frame::McpReadResource(request) => {
            let answer = match mcp_proxy.read_resource(request.0).await {
                Ok(result) => mcp_read_resource::Frame::Result(result),
                Err(error) => mcp_read_resource::Frame::Error(error),
            };
            answer_with(&handle, scope, channel, &answer).await
        }
        channel_request::Frame::McpNotifications(_) => {
            notify(&handle, scope, channel, &*mcp_proxy).await
        }
        channel_request::Frame::FetchFile(request) => {
            fetched_file(&handle, scope, channel, &*fetch_proxy, request)
                .await
        }
        channel_request::Frame::FetchDirectory(request) => {
            fetched_directory(
                &handle,
                scope,
                channel,
                &*fetch_proxy,
                request,
            )
            .await
        }
        channel_request::Frame::FetchResource(request) => {
            fetched_resource(&handle, scope, channel, &*fetch_proxy, request)
                .await
        }
    };

    let _ = handle.send_channel_response_finish(scope, channel).await;
}

/// Relay a server's notifications until it stops saying things.
///
/// One frame each, for as long as the stream lasts. Unlike the other
/// four this is not an answer — it is the place a server pushes into,
/// and it ends when the server has no more to push or when the caller
/// can no longer listen.
///
/// # An error is the last one
///
/// It goes out like any other frame and then the stream is dropped,
/// because there is nothing after a stream that stopped. Dropping it is
/// also how a caller stops listening: there is no unsubscribe, and the
/// channel finishing is one.
async fn notify<P>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    mcp_proxy: &P,
) -> bool
where
    P: McpProxy,
{
    let mut notifications = mcp_proxy.notifications().await;
    while let Some(item) = notifications.next().await {
        // Asked before the value moves into the frame.
        let last = item.is_err();
        let frame = match item {
            Ok(notification) => {
                mcp_notifications::Frame::Notification(notification)
            }
            Err(error) => mcp_notifications::Frame::Error(error),
        };
        if !answer_with(handle, scope, channel, &frame).await {
            return false;
        }
        if last {
            break;
        }
    }
    true
}

/// Send a fetched file, one chunk per frame.
///
/// The proxy yields owned bytes; each borrows into the exchange's
/// frame as it is written, so nothing is copied on the way out. The
/// stream ending is the whole of "the file is complete", and an
/// empty stream sends nothing at all — the finish the caller sends
/// afterwards is then the empty finish, which is how a client says
/// it does not hold the identity. Nothing here can tell those apart,
/// and nothing needs to: both are the stream being over.
async fn fetched_file<F>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    fetch_proxy: &F,
    request: channel_request::fetch_file::Request,
) -> bool
where
    F: FetchProxy,
{
    let mut chunks = fetch_proxy.fetch_file(request.identity).await;
    while let Some(chunk) = chunks.next().await {
        let frame = fetch_file::Frame { body: &chunk };
        if !answer_with(handle, scope, channel, &frame).await {
            return false;
        }
    }
    true
}

/// Send a fetched directory, one file — or one chunk of one — per
/// frame.
///
/// The proxy yields owned paths and bytes; each borrows into the
/// exchange's frame as it is written. The stream ending is the whole
/// of "the directory is complete", and an empty stream sends nothing
/// at all — the empty finish, the client saying it does not hold the
/// identity.
async fn fetched_directory<F>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    fetch_proxy: &F,
    request: channel_request::fetch_directory::Request,
) -> bool
where
    F: FetchProxy,
{
    let mut files = fetch_proxy.fetch_directory(request.identity).await;
    while let Some((path, body)) = files.next().await {
        let frame = fetch_directory::Frame { path, body: &body };
        if !answer_with(handle, scope, channel, &frame).await {
            return false;
        }
    }
    true
}

/// Send a fetched resource, one chunk per frame.
///
/// [`fetched_file`]'s shape exactly, because a resource IS bytes
/// under the file identity — the stream ending says the resource is
/// complete, and an empty stream leaves the empty finish, the
/// client saying it does not hold the identity.
async fn fetched_resource<F>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    fetch_proxy: &F,
    request: channel_request::fetch_resource::Request,
) -> bool
where
    F: FetchProxy,
{
    let mut chunks = fetch_proxy.fetch_resource(request.identity).await;
    while let Some(chunk) = chunks.next().await {
        let frame = fetch_resource::Frame { body: &chunk };
        if !answer_with(handle, scope, channel, &frame).await {
            return false;
        }
    }
    true
}

/// Encode one answer and write it.
///
/// Generic over the frame because the seven exchanges answer with
/// seven types, and what happens to each of them here is identical:
/// build it, write it, stop if it did not go.
///
/// Answers whether to carry on, so the caller knows whether a finish is
/// still worth sending.
///
/// A frame that will not encode is treated as a refusal rather than
/// handled. There is nowhere to report one — the channel for saying so
/// is the thing that would not serialize — and stopping is what a
/// caller would do about it anyway.
async fn answer_with<F>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    frame: &F,
) -> bool
where
    F: Encode,
{
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_err() {
        return false;
    }
    handle
        .send_channel_response(scope, channel, &buffer)
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
