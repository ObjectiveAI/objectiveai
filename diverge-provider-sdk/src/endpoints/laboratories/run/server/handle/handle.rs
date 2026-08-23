//! Running a laboratory, and serving everyone it concerns.

use std::net::IpAddr;
use std::pin::pin;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::future::{self, Either};
use futures_util::{Stream, StreamExt as _, stream};
use indexmap::IndexMap;
use rmcp::model::{
    CallToolRequestParams, PaginatedRequestParams, ReadResourceRequestParams,
};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use super::super::{channel_request, channel_response, response};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::laboratories::run::client::channel_request as asked;
use crate::endpoints::laboratories::run::client::channel_response as answered;
use crate::endpoints::laboratories::run::client::request;
use crate::frame::client::ClientFrame;
use crate::server::channel::Channel;
use crate::server::client_registry::ClientRegistry;
use crate::server::container::{Container, ContentError};
use crate::server::container_deployer::ContainerDeployer;
use crate::server::deployment::Deployment;
use crate::server::laboratories::{
    Event, Laboratories, Laboratory, MCP_PORT,
};
use crate::server::mount;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::container::request::Image;
use crate::shared::container::{read, transfer, write_bytes, write_path};
use crate::shared::error::Error;

/// Run a laboratory, and keep serving it until somebody stops.
///
/// The handler with the most parties. The caller opens channels for
/// MCP exchanges, reads, writes and transfers; the provider asks the
/// caller for an image's bytes, a connector's authorization and a
/// write's content; and connectors — on scopes of their own, possibly
/// on other connections entirely — reach this container through the
/// [`Laboratories`] registry and speak to this handler through its
/// events.
///
/// # What goes down the scope
///
/// The container's id, its filesystem and its departures, interleaved
/// however they happen — see
/// [`response::Frame`] for why nothing is ordered. The one thing that
/// ends the scope from this side is the deploy failing; after that the
/// scope lives until the caller stops it or leaves.
///
/// # The container is stopped on every path
///
/// Including the ones that failed. Teardown in this crate is a method,
/// so there is one call, after the serving, and every exit goes
/// through it.
///
/// # Stopping takes the connectors with it
///
/// A connection cannot outlive the thing it joined. The mechanism is
/// the event channel: this handler owns the receiver, connectors hold
/// senders, and the receiver dropping is what their
/// [`closed`](mpsc::UnboundedSender::closed) resolves on — so ending
/// this run IS telling every connector, without this run knowing who
/// they are.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
pub async fn handle<D>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    deployer: &D,
    laboratories: &Laboratories<D::Container>,
) where
    D: ContainerDeployer,
    D::Error: Into<Error>,
    <D::Container as Container>::Error: Into<Error>,
{
    // Shared from here, because the workers write on it and none of
    // them may hold it alone. What stays exclusive is ENDING the scope,
    // which is why the finish has to get the handle back out.
    let scope = Arc::new(scope);

    let deployment = Deployment {
        memory: request.memory,
        disk: request.disk,
        environment: environment(&request),
        mounts: mounts(client_identity, &request),
        ports: vec![MCP_PORT],
    };

    let container = match deploy(
        &scope,
        deployer,
        client_identity,
        &deployment,
        &request.image,
    )
    .await
    {
        Ok(container) => Arc::new(container),
        Err(error) => {
            write(&scope, &response::Frame::Error(error)).await;
            finish(scope).await;
            return;
        }
    };

    // Random because it is a capability: knowing the id is what
    // entitles a caller to name this laboratory in a connect or a
    // transfer, so an id anyone could predict would be a container
    // anyone could reach.
    let id = Uuid::new_v4().to_string();
    let (event_sender, event_receiver) = mpsc::unbounded_channel();
    laboratories
        .insert(
            id.clone(),
            Laboratory {
                container: Arc::clone(&container),
                events: event_sender,
            },
        )
        .await;
    // After the insert, so nobody can quote an id the registry cannot
    // yet resolve.
    write(&scope, &response::Frame::Id(response::Id { id: id.clone() }))
        .await;

    serve(&scope, &container, laboratories, event_receiver).await;

    // The receiver died when `serve` returned, which is the moment
    // every connector's `closed()` fired; the entry comes out here so
    // no NEW connect resolves a laboratory that is going away. A get
    // that races this and wins holds a dead sender, and a send on one
    // fails cleanly — nobody joins a stopping container either way.
    laboratories.remove(&id).await;
    container.stop().await;
    finish(scope).await;
}

/// What the container is told, and under what names.
///
/// The caller's [`environment`](request::Frame::environment) first,
/// then this crate's own names over the top — so a caller cannot
/// shadow one by guessing it, and whatever inside the container reads
/// one is reading what the protocol delivered rather than what
/// somebody set.
///
/// # The two reserved names
///
/// `DIVERGE_LABORATORY_NAME` carries
/// [`name`](request::Frame::name) verbatim, and
/// `DIVERGE_LABORATORY_INITIAL_CWD` carries
/// [`initial_cwd`](request::Frame::initial_cwd) as a JSON array of its
/// components. An array rather than a joined string, because a path is
/// a sequence and joining one invents a separator that then has to be
/// escaped out of names containing it — the same reason the field
/// itself is components.
///
/// They exist for the provider's own laboratory server, which is the
/// thing inside the container that wants to know what it is called and
/// where an agent should land. Neither is a container fact, which is
/// why they arrive this way: a provider "delivers them through the
/// environment by its own reserved names", and a handler here is the
/// provider for that purpose.
fn environment(request: &request::Frame) -> IndexMap<String, String> {
    let mut environment = request.environment.clone();
    environment.insert(
        "DIVERGE_LABORATORY_NAME".to_owned(),
        request.name.clone(),
    );
    // Through `Value`, whose `Display` IS its JSON: a `Vec<String>`
    // cannot fail to serialize, and this is the spelling that does not
    // return a `Result` nothing could be done with.
    environment.insert(
        "DIVERGE_LABORATORY_INITIAL_CWD".to_owned(),
        Value::from(request.initial_cwd.clone()).to_string(),
    );
    environment
}

/// The caller's mounts, with the caller attached.
///
/// A [`Mount`](crate::shared::container::request::Mount) names a
/// volume, and a volume's name is unique within the caller it was
/// listed to — so what a deployer needs is the name together with who
/// asked, which is a fact only this half of the connection has. See
/// [`mount::Mount`].
fn mounts(client_identity: &str, request: &request::Frame) -> Vec<mount::Mount> {
    request
        .mounts
        .iter()
        .map(|mount| mount::Mount {
            client_identity: client_identity.to_owned(),
            host_name: mount.host_name.clone(),
            host_relative_path: mount.host_relative_path.clone(),
            container_path: mount.container_path.clone(),
        })
        .collect()
}

/// Put the container somewhere, by whichever route its image came
/// from.
///
/// The three [`Image`] variants and the three
/// [`ContainerDeployer`] methods are the same three things, which is
/// what lets the fourth argument exist: only a
/// [`Client`](Image::Client) image needs somewhere to ask for bytes,
/// so only that method is handed a [`ClientRegistry`].
async fn deploy<D>(
    scope: &Arc<ScopeHandle>,
    deployer: &D,
    client_identity: &str,
    deployment: &Deployment,
    image: &Image,
) -> Result<D::Container, Error>
where
    D: ContainerDeployer,
    D::Error: Into<Error>,
{
    match image {
        Image::Client { name, digest } => {
            let registry = ClientRegistry::new(scope, |request, out| {
                channel_request::Frame::Oci(request).encode(out)
            });
            deployer
                .client(client_identity, deployment, name, digest, registry)
                .await
        }
        Image::Server { name, digest } => {
            deployer.server(client_identity, deployment, name, digest).await
        }
        Image::Registry { reference } => {
            deployer.registry(client_identity, deployment, reference).await
        }
    }
    .map_err(Into::into)
}

/// Serve the laboratory until it is stopped or the caller leaves.
///
/// One loop over two queues — what the caller opens, and what
/// connectors need said — a task per exchange, and a filetree pump
/// running beside all of it.
///
/// # Two queues, one loop
///
/// [`future::select`] rather than reading them in turn, because
/// neither may wait behind the other: a connector at the door is
/// blocked until its [`Event::Authorize`] is taken, and a caller's ask
/// deserves the same. Both are cancel-safe receives, so the loser of
/// each round loses nothing.
async fn serve<C>(
    scope: &Arc<ScopeHandle>,
    container: &Arc<C>,
    laboratories: &Laboratories<C>,
    mut events: mpsc::UnboundedReceiver<Event>,
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
        // laboratory.
        while workers.try_join_next().is_some() {}

        let arrived = {
            let ask = pin!(scope.recv_channel_request());
            let event = pin!(events.recv());
            match future::select(ask, event).await {
                Either::Left((bytes, _)) => Either::Left(bytes),
                Either::Right((event, _)) => Either::Right(event),
            }
        };

        let bytes = match arrived {
            // The caller is gone, and nothing it asked for matters
            // now.
            Either::Left(None) => break,
            Either::Left(Some(bytes)) => bytes,
            Either::Right(Some(Event::Authorize {
                address,
                authorization,
                reply,
            })) => {
                // On its own task: a runner deciding is somebody
                // waiting at a door, and a slow decision must delay
                // that connector and nothing else.
                workers.spawn(authorize(
                    Arc::clone(scope),
                    address,
                    authorization,
                    reply,
                ));
                continue;
            }
            Either::Right(Some(Event::Disconnected { nickname })) => {
                write(
                    scope,
                    &response::Frame::Disconnected(response::Disconnected {
                        nickname,
                    }),
                )
                .await;
                continue;
            }
            // Unreachable while the registry holds this channel's
            // sender, which it does until after this returns. If it
            // somehow closed, a run that can no longer hear its
            // connectors is a run that should end rather than spin on
            // a dead queue.
            Either::Right(None) => break,
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
            Ok(asked::Frame::Stop) => break,
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
/// [`Container::filetree`] promises a snapshot and then deltas, and
/// every item becomes a [`Filetree`](response::Frame::Filetree) frame.
///
/// A watch that cannot start, or that breaks, reports nothing more and
/// nothing else: the container is still fine, everything else still
/// serves, and the runner keeps whatever tree it last saw. A broken
/// watch is not a broken laboratory.
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

/// Ask the runner whether a connector may attach, and say what it
/// said.
///
/// One frame out on this scope, one answer back, and the verdict
/// through the oneshot: the nickname for a yes, [`None`] for anything
/// else. A frame that would not go out, an answer that never came and
/// an answer this crate could not read all collapse into [`None`],
/// because they should — a runner that could not be asked has not said
/// yes, and the connector at the door cannot act on the difference.
///
/// The oneshot dropping unanswered says the same thing, so the failure
/// paths simply return.
async fn authorize(
    scope: Arc<ScopeHandle>,
    address: IpAddr,
    authorization: String,
    reply: oneshot::Sender<Option<String>>,
) {
    let mut payload = Vec::new();
    let request = channel_request::Authorize {
        address,
        authorization,
    };
    if channel_request::Frame::Authorize(request)
        .encode(&mut Writer::new(&mut payload))
        .is_err()
    {
        return;
    }
    let mut channel = scope.send_channel_request(&payload).await;

    let Some(bytes) = channel.response_receiver.recv().await else {
        return;
    };
    let Ok(ClientFrame::ChannelResponse { payload, .. }) =
        ClientFrame::decode(&bytes)
    else {
        // A finish with nothing before it: the runner's executor could
        // not serve the ask at all.
        return;
    };
    let verdict = match answered::authorize::Frame::decode(payload) {
        Ok(answered::authorize::Frame::Authorized(nickname)) => {
            Some(nickname.to_owned())
        }
        Ok(answered::authorize::Frame::Denied) | Err(_) => None,
    };
    let _ = reply.send(verdict);
}

/// Answer one caller-opened channel, and finish it.
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
/// reason: the channel being over is a fact the caller cannot go
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

/// Ask the laboratory what tools it offers, for the caller.
///
/// The container's own MCP server answers, through
/// [`Container::mcp_list_tools`] on the port the provider gave it. Its
/// refusal is an answer — the `Error` frame — where the container
/// being unreachable is not, and [`answer`] says what each becomes on
/// the wire.
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

/// Ask the laboratory what resources it offers, for the caller.
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

/// Run one of the laboratory's tools, for the caller.
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

/// Read one of the laboratory's resources, for the caller.
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

/// Write one file into the container, from content the caller serves.
///
/// The write's two channels meet here: the caller asked on one, and
/// the content arrives on a second this opens back — see
/// [`content`] for how the second becomes the stream
/// [`Container::write`] consumes. The answer goes where the ask came
/// from, and that channel is finished on every path.
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
    let ask = write_bytes::request::Request {
        write_id: request.write_id,
    };
    let answer = match channel_request::Frame::Write(ask)
        .encode(&mut Writer::new(&mut payload))
    {
        Ok(()) => {
            let opened = scope.send_channel_request(&payload).await;
            container
                .write(&request.path, Box::pin(content(opened)))
                .await
                .map_err(Into::into)
        }
        // Four known bytes that did not serialize would be this frame's
        // JSON variant failing, which the Write variant is not — but
        // the type carries the union, so the arm exists and says what
        // happened.
        Err(error) => Err(Error(Value::String(error.to_string()))),
    };

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

/// A write's content, off the channel the caller answers into.
///
/// The adapter between the wire and [`Container::write`]: `Body`
/// frames become `Ok` pieces, refcounted out of the frames they
/// arrived in; the caller's `Error` becomes
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

/// Copy one file from this container into another, for the caller.
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

/// One answer frame onto a caller's channel, then the finish.
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
