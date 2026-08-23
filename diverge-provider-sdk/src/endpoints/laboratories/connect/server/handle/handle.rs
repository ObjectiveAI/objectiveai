//! Joining a running laboratory, and serving the connector inside it.

use std::net::IpAddr;
use std::pin::pin;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::future::{self, Either};
use futures_util::{Stream, StreamExt as _, stream};
use rmcp::model::{
    CallToolRequestParams, PaginatedRequestParams, ReadResourceRequestParams,
};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use super::super::{channel_request, channel_response, response};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::laboratories::connect::client::channel_request as asked;
use crate::endpoints::laboratories::connect::client::channel_response as answered;
use crate::endpoints::laboratories::connect::client::request;
use crate::frame::client::ClientFrame;
use crate::server::channel::Channel;
use crate::server::container::{Container, ContentError};
use crate::server::laboratories::{Event, Laboratories, MCP_PORT};
use crate::server::scope_handle::ScopeHandle;
use crate::shared::container::{read, transfer, write_bytes, write_path};
use crate::shared::error::Error;

/// Join a laboratory somebody else is running, and serve the connector
/// until either of them is done.
///
/// The one handler that deploys nothing. The container already exists,
/// on somebody else's scope; this resolves the id through the
/// [`Laboratories`] registry, asks that scope's runner whether the
/// connector may attach, and then serves the same nine asks a runner
/// gets — against a container it does not own and will not stop.
///
/// # `address` is the provider's to supply
///
/// Where the connector's socket comes from, as the provider sees it.
/// It rides the authorization to the runner verbatim, and the
/// [`Authorize`](crate::endpoints::laboratories::run::server::channel_request::Authorize)
/// it lands in says exactly how much it may be trusted: a signal, not
/// an identity.
///
/// # Every arrival the runner granted is a departure it hears
///
/// The runner's answer to an authorization carries the nickname it
/// chose, and "nothing can disconnect that was not authorized first"
/// is half of an invariant — this handler keeps the other half. From
/// the moment the runner says yes, EVERY way out of here tells the
/// run handler the connector left, under that nickname: the graceful
/// [`Disconnect`](asked::Frame::Disconnect), the connector vanishing,
/// the laboratory itself ending. A runner that saw four arrive sees
/// four leave.
///
/// # Leaving takes nothing with it
///
/// No container stop, no registry removal. The container goes on
/// running, other connectors stay attached, and the runner sees one
/// fewer connection — which is the whole difference between a
/// connector and a runner.
pub async fn handle<C>(
    scope: ScopeHandle,
    address: IpAddr,
    laboratories: &Laboratories<C>,
) where
    C: Container + 'static,
    C::Error: Into<Error>,
{
    // Shared from here, because the workers write on it and none of
    // them may hold it alone. What stays exclusive is ENDING the scope,
    // which is why the finish has to get the handle back out.
    let scope = Arc::new(scope);

    let request = match request::Frame::decode(scope.request()) {
        Ok(request) => request,
        Err(error) => {
            let error = Error(Value::String(error.to_string()));
            write(&scope, &response::Frame::Error(error)).await;
            finish(scope).await;
            return;
        }
    };

    // Holding the id is what entitles the connector to ASK; whether it
    // may attach is still the runner's answer, below. An id the
    // registry cannot resolve is a laboratory that is not running —
    // never was, or already ended, which are indistinguishable from
    // here and should be.
    let Some((container, events)) = laboratories.get(&request.id).await
    else {
        let error = "no laboratory has that id";
        let error = Error(Value::String(error.to_owned()));
        write(&scope, &response::Frame::Error(error)).await;
        finish(scope).await;
        return;
    };

    let (reply_sender, reply_receiver) = oneshot::channel();
    let asked_runner = events
        .send(Event::Authorize {
            address,
            authorization: request.authorization,
            reply: reply_sender,
        })
        .is_ok();
    // A send that failed and a reply that never comes are the run
    // ending under us, and a denial reads the same from this side: the
    // runner has not said yes.
    let nickname = match asked_runner {
        true => reply_receiver.await.ok().flatten(),
        false => None,
    };
    let Some(nickname) = nickname else {
        let error = "the runner did not authorize the connection";
        let error = Error(Value::String(error.to_owned()));
        write(&scope, &response::Frame::Error(error)).await;
        finish(scope).await;
        return;
    };

    serve(&scope, &container, laboratories, &events).await;

    // The other half of the invariant: the runner heard this connector
    // arrive, so it hears it leave, on every path past the yes. A send
    // that fails is the run already gone, and a runner that is gone is
    // owed nothing.
    let _ = events.send(Event::Disconnected { nickname });
    finish(scope).await;
}

/// Serve the connector until it leaves, or the laboratory ends, or the
/// connection goes.
///
/// One loop over two facts — what the connector opens, and whether the
/// laboratory still exists — a task per exchange, and a filetree pump
/// running beside all of it.
///
/// # The second fact is a closed channel
///
/// [`closed`](mpsc::UnboundedSender::closed) resolves when the run
/// handler drops its receiver, which it does however the run ends. A
/// connection cannot outlive the thing it joined, and this is where
/// that stops being a rule and starts being what the select does.
async fn serve<C>(
    scope: &Arc<ScopeHandle>,
    container: &Arc<C>,
    laboratories: &Laboratories<C>,
    events: &mpsc::UnboundedSender<Event>,
) where
    C: Container + 'static,
    C::Error: Into<Error>,
{
    // Owned here, so that leaving this function cancels everything it
    // started rather than leaving tasks holding a scope that is about
    // to finish.
    let mut workers = tokio::task::JoinSet::new();
    workers.spawn(filetree(Arc::clone(container), Arc::clone(scope)));

    loop {
        // Finished ones, so the set does not grow for the life of the
        // connection.
        while workers.try_join_next().is_some() {}

        let bytes = {
            let ask = pin!(scope.recv_channel_request());
            let over = pin!(events.closed());
            match future::select(ask, over).await {
                // The connector is gone, and nothing it asked for
                // matters now.
                Either::Left((None, _)) => break,
                Either::Left((Some(bytes), _)) => bytes,
                // The laboratory ended. The container is the run
                // handler's to stop, and this connection's business is
                // only to be over.
                Either::Right(((), _)) => break,
            }
        };

        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) =
            ClientFrame::decode(&bytes)
        else {
            continue;
        };

        // The MCP variants carry owned params, so each ask moves into
        // its task whole — nothing borrows the frame it arrived in.
        match asked::Frame::decode(payload) {
            Ok(asked::Frame::McpListTools(request)) => {
                workers.spawn(mcp_list_tools(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    request.0,
                ));
            }
            Ok(asked::Frame::McpListResources(request)) => {
                workers.spawn(mcp_list_resources(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    request.0,
                ));
            }
            Ok(asked::Frame::McpCallTool(request)) => {
                workers.spawn(mcp_call_tool(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    request.0,
                ));
            }
            Ok(asked::Frame::McpReadResource(request)) => {
                workers.spawn(mcp_read_resource(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    request.0,
                ));
            }
            Ok(asked::Frame::McpNotifications(_)) => {
                workers.spawn(mcp_notifications(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                ));
            }
            Ok(asked::Frame::Read(request)) => {
                workers.spawn(serve_read(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    request.path,
                ));
            }
            Ok(asked::Frame::Write(request)) => {
                workers.spawn(serve_write(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    request,
                ));
            }
            Ok(asked::Frame::Transfer(request)) => {
                // Resolved here rather than in the task, because the
                // registry is borrowed and a task must own everything
                // it holds. A lookup is a lock and two clones, short
                // enough to sit in the loop.
                let destination =
                    laboratories.get(&request.destination_id).await;
                workers.spawn(serve_transfer(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    request,
                    destination.map(|(container, _)| container),
                ));
            }
            Ok(asked::Frame::Disconnect) => break,
            // A channel this end cannot read is ENDED rather than
            // abandoned: a caller waiting on one waits forever, and a
            // finish with nothing before it already means there was no
            // answer.
            Err(_) => scope.send_channel_response_finish(channel).await,
        }
    }

    // Aborted AND awaited. Dropping the set would abort without
    // waiting, and every worker holds a share of the scope — so the
    // finish that follows could find shares still outstanding and end
    // the scope without a finish frame at all.
    workers.shutdown().await;
}

/// Report the container's filesystem, for as long as it reports.
///
/// The connector's own subscription — [`Container::filetree`] begins
/// every stream with a snapshot, which is exactly what someone
/// arriving an hour into a run needs before any delta means anything.
///
/// A watch that cannot start, or that breaks, reports nothing more and
/// nothing else: everything else still serves, and the connector keeps
/// whatever tree it last saw.
async fn filetree<C>(container: Arc<C>, scope: Arc<ScopeHandle>)
where
    C: Container,
{
    let Ok(mut frames) = container.filetree().await else {
        return;
    };
    while let Some(frame) = frames.next().await {
        write(&scope, &response::Frame::Filetree(frame)).await;
    }
}

/// Answer one connector-opened channel, and finish it.
///
/// The unary MCP shape, written once: at most one frame, then the
/// finish, on every path.
///
/// [`None`] is the container not being reachable, and it sends nothing
/// at all — a finish with nothing before it is already what the wire
/// means by "the provider could not serve the exchange", and the
/// caller's own error type names it `Unanswered`.
///
/// An encode failure sends nothing and still finishes, for the same
/// reason: the channel being over is a fact the connector cannot go
/// without, and gating it on serialization would leave a healthy
/// connection with a channel nobody can ever close.
async fn answer<F>(scope: &ScopeHandle, channel: u32, frame: Option<F>)
where
    F: Encode<Error = serde_json::Error>,
{
    if let Some(frame) = frame {
        let mut payload = Vec::new();
        if frame.encode(&mut Writer::new(&mut payload)).is_ok() {
            scope.send_channel_response(channel, &payload).await;
        }
    }
    scope.send_channel_response_finish(channel).await;
}

/// Ask the laboratory what tools it offers, for the connector.
///
/// The container's own MCP server answers, through
/// [`Container::mcp_list_tools`] on the port the provider gave it —
/// the same server the runner talks to, because a connector sees what
/// the runner sees. Its refusal is an answer — the `Error` frame —
/// where the container being unreachable is not, and [`answer`] says
/// what each becomes on the wire.
///
/// The three siblings below are this exchange against a different
/// noun, and differ in nothing else.
async fn mcp_list_tools<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    params: Option<PaginatedRequestParams>,
) where
    C: Container,
{
    let frame = match container.mcp_list_tools(MCP_PORT, params).await {
        Ok(Ok(result)) => {
            Some(channel_response::mcp_list_tools::Frame::Result(result))
        }
        Ok(Err(error)) => {
            Some(channel_response::mcp_list_tools::Frame::Error(error))
        }
        Err(_) => None,
    };
    answer(&scope, channel, frame).await;
}

/// Ask the laboratory what resources it offers, for the connector.
async fn mcp_list_resources<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    params: Option<PaginatedRequestParams>,
) where
    C: Container,
{
    let frame = match container.mcp_list_resources(MCP_PORT, params).await {
        Ok(Ok(result)) => {
            Some(channel_response::mcp_list_resources::Frame::Result(result))
        }
        Ok(Err(error)) => {
            Some(channel_response::mcp_list_resources::Frame::Error(error))
        }
        Err(_) => None,
    };
    answer(&scope, channel, frame).await;
}

/// Run one of the laboratory's tools, for the connector.
async fn mcp_call_tool<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    params: CallToolRequestParams,
) where
    C: Container,
{
    let frame = match container.mcp_call_tool(MCP_PORT, params).await {
        Ok(Ok(result)) => {
            Some(channel_response::mcp_call_tool::Frame::Result(result))
        }
        Ok(Err(error)) => {
            Some(channel_response::mcp_call_tool::Frame::Error(error))
        }
        Err(_) => None,
    };
    answer(&scope, channel, frame).await;
}

/// Read one of the laboratory's resources, for the connector.
async fn mcp_read_resource<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    params: ReadResourceRequestParams,
) where
    C: Container,
{
    let frame = match container.mcp_read_resource(MCP_PORT, params).await {
        Ok(Ok(result)) => {
            Some(channel_response::mcp_read_resource::Frame::Result(result))
        }
        Ok(Err(error)) => {
            Some(channel_response::mcp_read_resource::Frame::Error(error))
        }
        Err(_) => None,
    };
    answer(&scope, channel, frame).await;
}

/// Relay what the laboratory says on its own account, for as long as
/// it says anything.
///
/// The one MCP exchange that is not answered once. The channel stays
/// open and every frame on it is another notification, until the
/// container's stream ends — or says why it will push no more, which
/// is the `Error` frame and is terminal by that frame's own contract.
///
/// # The channel is finished on every path
///
/// Including after the error frame. The error says why the
/// notifications stopped; the finish says the channel is over, and
/// they are different facts on this wire.
async fn mcp_notifications<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
) where
    C: Container,
{
    let mut notifications = match container.mcp_notifications(MCP_PORT).await
    {
        Ok(notifications) => notifications,
        Err(_) => {
            scope.send_channel_response_finish(channel).await;
            return;
        }
    };

    let mut buffer = Vec::new();
    while let Some(item) = notifications.next().await {
        let (frame, last) = match item {
            Ok(notification) => (
                channel_response::mcp_notifications::Frame::Notification(
                    notification,
                ),
                false,
            ),
            Err(error) => (
                channel_response::mcp_notifications::Frame::Error(error),
                true,
            ),
        };
        buffer.clear();
        // A notification that will not serialize is dropped and the
        // stream goes on: it is one thing the container said, and the
        // next may be fine.
        if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
            scope.send_channel_response(channel, &buffer).await;
        }
        if last {
            break;
        }
    }
    scope.send_channel_response_finish(channel).await;
}

/// Read one file out of the container, onto the channel that asked.
///
/// Pieces as they come, then the finish. An [`Err`] item is the read
/// stopping — refused or truncated, which the frame deliberately does
/// not distinguish — and it is the last thing sent, because whatever
/// arrived before it is a prefix of the file and nothing after it
/// could change that.
async fn serve_read<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    path: Vec<String>,
) where
    C: Container,
    C::Error: Into<Error>,
{
    let mut file = container.read(&path).await;
    let mut buffer = Vec::new();
    while let Some(piece) = file.next().await {
        buffer.clear();
        match piece {
            Ok(bytes) => {
                // Breaking rather than returning on an encode that
                // failed: nothing went out, so the connection is still
                // working and the channel is still owed its finish.
                if channel_response::read::Frame::Body(
                    read::response::Frame(&bytes),
                )
                .encode(&mut Writer::new(&mut buffer))
                .is_err()
                {
                    break;
                }
                scope.send_channel_response(channel, &buffer).await;
            }
            Err(error) => {
                if channel_response::read::Frame::Error(error.into())
                    .encode(&mut Writer::new(&mut buffer))
                    .is_ok()
                {
                    scope.send_channel_response(channel, &buffer).await;
                }
                break;
            }
        }
    }
    scope.send_channel_response_finish(channel).await;
}

/// Write one file into the container, from content the connector
/// serves.
///
/// The write's two channels meet here: the connector asked on one, and
/// the content arrives on a second this opens back — see [`content`]
/// for how the second becomes the stream [`Container::write`]
/// consumes. The answer goes where the ask came from, and that channel
/// is finished on every path.
///
/// Unlike the runner's version of this exchange, the ask back is this
/// endpoint's whole channel-request vocabulary — a connector is asked
/// for a write's content and for nothing else — so the frame is a
/// tuple and encoding it cannot fail.
async fn serve_write<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    request: write_path::request::Request,
) where
    C: Container,
    C::Error: Into<Error>,
{
    let mut payload = Vec::new();
    channel_request::Frame(write_bytes::request::Request {
        write_id: request.write_id,
    })
    .encode(&mut Writer::new(&mut payload))
    .unwrap_or_else(|error| match error {});
    let opened = scope.send_channel_request(&payload).await;

    let answer = container
        .write(&request.path, Box::pin(content(opened)))
        .await
        .map_err(Into::into);

    let frame = match answer {
        Ok(()) => {
            channel_response::write_path::Frame::Written(
                write_path::response::Frame,
            )
        }
        Err(error) => channel_response::write_path::Frame::Error(error),
    };
    buffer_answer(&scope, channel, &frame).await;
}

/// A write's content, off the channel the connector answers into.
///
/// The adapter between the wire and [`Container::write`]: `Body`
/// frames become `Ok` pieces, refcounted out of the frames they
/// arrived in; the connector's `Error` becomes
/// [`ContentError::Wire`] and ends it; the channel finishing is the
/// content being complete and ends the stream cleanly. A channel that
/// closes WITHOUT a finish is a connection that went, which is also a
/// [`ContentError::Wire`] — the source saying, through its absence,
/// that it has no more to give.
///
/// The [`Channel`] rides inside and drops with the stream, which is
/// what tells the session that channel is over.
fn content<E>(
    channel: Channel,
) -> impl Stream<Item = Result<Bytes, ContentError<E>>> + Send + 'static {
    stream::unfold(Some(channel), |state| async move {
        let mut channel = state?;
        let Some(bytes) = channel.response_receiver.recv().await else {
            let error = "the connection ended before the content did";
            let error = Error(Value::String(error.to_owned()));
            return Some((Err(ContentError::Wire(error)), None));
        };
        let Ok(ClientFrame::ChannelResponse { payload, .. }) =
            ClientFrame::decode(&bytes)
        else {
            // The finish: the content is complete.
            return None;
        };
        match answered::write_bytes::Frame::decode(payload) {
            Ok(answered::write_bytes::Frame::Body(piece)) => {
                Some((Ok(bytes.slice_ref(piece.0)), Some(channel)))
            }
            Ok(answered::write_bytes::Frame::Error(error)) => {
                Some((Err(ContentError::Wire(error)), None))
            }
            Err(error) => {
                let error = Error(Value::String(error.to_string()));
                Some((Err(ContentError::Wire(error)), None))
            }
        }
    })
}

/// Copy one file from this container into another, for the connector.
///
/// A read here and a write there, and the destination is whichever
/// laboratory the id named when the ask arrived — resolved by the
/// dispatch loop, because holding the id is the whole of the
/// authorization and the registry is the only thing that can say what
/// it names. [`None`] is an id naming nothing, which is its own
/// answer.
async fn serve_transfer<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    source: Arc<C>,
    request: transfer::request::Request,
    destination: Option<Arc<C>>,
) where
    C: Container,
    C::Error: Into<Error>,
{
    let answer = match destination {
        Some(destination) => {
            let file = source.read(&request.path).await;
            // The source's failures are the provider's own vocabulary,
            // which is what `Container` distinguishes from a caller's
            // content stopping.
            let file = file.map(|piece| {
                piece.map_err(ContentError::Container)
            });
            destination
                .write(&request.destination_path, Box::pin(file))
                .await
                .map_err(Into::into)
        }
        None => {
            let error = "no laboratory has that id";
            Err(Error(Value::String(error.to_owned())))
        }
    };

    let frame = match answer {
        Ok(()) => {
            channel_response::transfer::Frame::Transferred(
                transfer::response::Frame,
            )
        }
        Err(error) => channel_response::transfer::Frame::Error(error),
    };
    buffer_answer(&scope, channel, &frame).await;
}

/// One answer frame onto a connector's channel, then the finish.
///
/// The write and transfer answers share it: encode, send if that
/// worked, and finish regardless — an encode failure must not leave
/// the channel open, for the reason [`answer`] gives.
async fn buffer_answer<F>(scope: &ScopeHandle, channel: u32, frame: &F)
where
    F: Encode<Error = serde_json::Error>,
{
    let mut payload = Vec::new();
    if frame.encode(&mut Writer::new(&mut payload)).is_ok() {
        scope.send_channel_response(channel, &payload).await;
    }
    scope.send_channel_response_finish(channel).await;
}

/// End the scope, which needs the handle back to itself.
///
/// [`send_response_finish`](ScopeHandle::send_response_finish)
/// consumes the handle, which is what makes one finish per scope a
/// fact rather than a rule — so it cannot be reached through a share,
/// and this is where the shares are proved gone.
async fn finish(scope: Arc<ScopeHandle>) {
    if let Some(scope) = Arc::into_inner(scope) {
        scope.send_response_finish().await;
    }
}

/// Write one frame, or write nothing if it will not encode.
async fn write(scope: &ScopeHandle, frame: &response::Frame) {
    let mut bytes = Vec::new();
    if frame.encode(&mut Writer::new(&mut bytes)).is_ok() {
        scope.send_response(&bytes).await;
    }
}
