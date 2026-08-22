//! Running a plugin, and serving everything either end asks of it.

use std::collections::HashMap;
use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use futures_util::{SinkExt as _, StreamExt as _};
use http_body_util::{BodyStream, Full};
use indexmap::IndexMap;
use serde_json::Value;
use tokio::sync::{Mutex, mpsc};

use super::super::{channel_request, channel_response, response};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::mcp_plugin::run::client::channel_request as asked;
use crate::endpoints::mcp_plugin::run::client::channel_response as answered;
use crate::endpoints::mcp_plugin::run::client::request;
use crate::frame::client::ClientFrame;
use crate::server::channel::Channel;
use crate::server::client_registry::ClientRegistry;
use crate::server::container::Container;
use crate::server::container_deployer::ContainerDeployer;
use crate::server::deployment::Deployment;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::container::request::Image;
use crate::shared::error::Error;
use crate::shared::http;

/// Run the plugin, and keep serving it until somebody stops.
///
/// The largest handler, and the only one where both ends ask things of
/// the provider. A caller opens channels for MCP exchanges and for the
/// plugin's database writes; the plugin, from inside its container,
/// asks for database connections and for commands to be run. None of it
/// waits on any of the rest.
///
/// # Silence is the whole of a working run
///
/// Every other handler answers and finishes. This one opens the scope
/// and says nothing at all for the container's entire life — the only
/// [`response::Frame`] it can send is the error saying the plugin did
/// not come up, and a caller that receives none has a plugin that is
/// running. See that type for why there is no readiness signal to send
/// instead.
///
/// # It takes a `client_identity`
///
/// Which is not the plugin's own [`Identity`](request::Identity). That
/// one is what the CALLER says about itself and reaches the container
/// as environment; this is what the PROVIDER established about the
/// connection, and it is here because pulling a
/// [`Client`](Image::Client) image happens on that caller's behalf.
pub async fn handle<D>(scope: ScopeHandle, client_identity: &str, deployer: &D)
where
    D: ContainerDeployer,
    D::Error: Into<Error>,
    <D::Container as Container>::Error: Into<Error>,
{
    // Nothing consults it yet. It is an argument because the deploy
    // below is done for somebody, and a provider that attributes work
    // needs to be told whose it is.
    let _ = client_identity;

    // Shared from here, because the workers write on it and none of
    // them may hold it alone. What stays exclusive is ENDING the scope,
    // which is why the finish has to get the handle back out.
    let scope = Arc::new(scope);

    let request = match request::Frame::decode(scope.request()) {
        Ok(request) => request,
        Err(error) => {
            let error = Error(Value::String(error.to_string()));
            write(&scope, &response::Frame(error)).await;
            finish(scope).await;
            return;
        }
    };

    let mut ports = vec![request.mcp_port];
    ports.extend(request.postgres_port);
    ports.extend(request.command_port);

    let deployment = Deployment {
        memory: request.memory,
        disk: request.disk,
        environment: environment(&request),
        // A plugin serves tools rather than works on a filesystem, and
        // mounting a caller's directories into one would hand it access
        // it has no reason to want.
        mounts: Vec::new(),
        ports,
    };

    let container =
        match deploy(&scope, deployer, &deployment, &request.image).await {
            Ok(container) => Arc::new(container),
            Err(error) => {
                write(&scope, &response::Frame(error)).await;
                finish(scope).await;
                return;
            }
        };

    serve(&scope, &container, &request).await;

    container.stop().await;
    finish(scope).await;
}

/// Put the container somewhere, by whichever route its image came from.
///
/// The three [`Image`] variants and the three
/// [`ContainerDeployer`] methods are the same three things, which is
/// what lets the fourth argument exist: only a
/// [`Client`](Image::Client) image needs somewhere to ask for bytes, so
/// only that method is handed a [`ClientRegistry`].
async fn deploy<D>(
    scope: &Arc<ScopeHandle>,
    deployer: &D,
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
            deployer.client(deployment, name, digest, registry).await
        }
        Image::Server { name, digest } => {
            deployer.server(deployment, name, digest).await
        }
        Image::Registry { reference } => {
            deployer.registry(deployment, reference).await
        }
    }
    .map_err(Into::into)
}

/// What the container is told, and under what names.
///
/// The caller's [`environment`](request::Frame::environment) first,
/// then this crate's own names over the top — so a caller cannot shadow
/// one by guessing it, and a plugin reading one is reading what the
/// protocol delivered rather than what somebody set.
///
/// # Why arguments and identity arrive this way
///
/// Because neither is a container fact. [`Deployment`] says as much: a
/// provider "delivers them through the environment by its own reserved
/// names", and a handler here is the provider for that purpose, so the
/// names are settled here.
///
/// `DIVERGE_ARGUMENT_<NAME>` and `DIVERGE_IDENTITY_<FIELD>`, upper
/// cased. An argument's value is its JSON whatever shape it had — a
/// string arrives quoted, because unquoting one type and not the others
/// would make a plugin guess which happened.
///
/// An absent identity field sets no variable rather than an empty one,
/// which is the distinction the field itself makes: a caller that did
/// not say and a caller that said nothing are different, and only one
/// of them is a name in the environment.
fn environment(request: &request::Frame) -> IndexMap<String, String> {
    let mut environment = request.environment.clone();

    for (name, value) in &request.arguments {
        let name = format!("DIVERGE_ARGUMENT_{}", name.to_uppercase());
        environment.insert(name, value.to_string());
    }

    let identity = &request.identity;
    let mut set = |field: &str, value: Option<String>| {
        if let Some(value) = value {
            environment.insert(format!("DIVERGE_IDENTITY_{field}"), value);
        }
    };
    set("AGENT_INSTANCE", Some(identity.agent_instance.clone()));
    set(
        "AGENT_PARENT_INSTANCE",
        identity.agent_parent_instance.as_ref().map(|it| it.join(",")),
    );
    set("AGENT_ID", identity.agent_id.clone());
    set("AGENT_FULL_ID", identity.agent_full_id.clone());
    set("AGENT_REMOTE", identity.agent_remote.clone());
    set("AGENTIC_LOOP_ID", identity.agentic_loop_id.clone());
    set("PLUGIN_OWNER", identity.plugin_owner.clone());
    set("PLUGIN_NAME", identity.plugin_name.clone());
    set("PLUGIN_VERSION", identity.plugin_version.clone());
    set("TASK", identity.task.clone());

    environment
}

/// Where a database connection waits for the caller to come for it.
///
/// A connection is one pipe and TWO channels, and they arrive at
/// different moments: the provider mints the id and opens its own
/// channel the instant a pipe is taken, and the caller's channel
/// quoting that id turns up whenever the caller gets to it. So the
/// plugin's writes wait here in between.
///
/// The receiver is put in BEFORE the channel request that names the id
/// goes out, which is what makes the lookup safe: a caller cannot ask
/// about an id it has not been told, and by the time it has been told,
/// this holds it.
type Connections = Arc<Mutex<HashMap<u32, mpsc::UnboundedReceiver<Bytes>>>>;

/// Serve the plugin until it is stopped or the caller leaves.
///
/// Two conduits dialling ahead into the container, a task per exchange,
/// and this reading what the caller opens and handing each one on.
async fn serve<C>(
    scope: &Arc<ScopeHandle>,
    container: &Arc<C>,
    request: &request::Frame,
) where
    C: Container + 'static,
    C::Error: Into<Error>,
{
    let connections: Connections = Default::default();
    // Owned here, so that leaving this function cancels everything it
    // started rather than leaving tasks holding a scope that is about
    // to finish.
    let mut workers = tokio::task::JoinSet::new();

    if let Some(port) = request.postgres_port {
        workers.spawn(postgres(
            Arc::clone(container),
            port,
            Arc::clone(scope),
            Arc::clone(&connections),
        ));
    }
    if let Some(port) = request.command_port {
        workers.spawn(command(Arc::clone(container), port, Arc::clone(scope)));
    }

    let mcp_port = request.mcp_port;
    loop {
        // The caller is gone, and nothing it asked for matters now.
        let Some(bytes) = scope.recv_channel_request().await else {
            break;
        };

        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) =
            ClientFrame::decode(&bytes)
        else {
            continue;
        };

        match asked::Frame::decode(payload) {
            Ok(asked::Frame::Mcp(_)) => {
                // The frame is re-decoded inside the task, because what
                // it holds borrows from bytes this loop owns.
                workers.spawn(mcp(
                    bytes.clone(),
                    channel,
                    mcp_port,
                    Arc::clone(scope),
                    Arc::clone(container),
                ));
            }
            Ok(asked::Frame::Stop) => break,
            Ok(asked::Frame::Postgres(postgres)) => {
                workers.spawn(writes(
                    postgres.connection_id,
                    channel,
                    Arc::clone(scope),
                    Arc::clone(&connections),
                ));
            }
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

/// One MCP exchange, relayed into the container and answered back.
///
/// The provider does not parse what it carries. It does not read
/// JSON-RPC, does not track sessions, and never looks at the
/// `Mcp-Session-Id` that ties a caller's exchanges together — the
/// headers go in as they came and come back the same way.
///
/// # One connection per exchange
///
/// Because that is what MCP over Streamable HTTP is: discrete requests
/// over a session identified by a header rather than by anything at the
/// transport layer. A pipe held open between them would be a pool this
/// crate would then be managing on the plugin's behalf.
///
/// # The channel is finished on every path
///
/// Including the ones with nothing to say. A caller that gets a finish
/// with no head knows the exchange produced nothing — which is exactly
/// what a wrong [`mcp_port`](request::Frame::mcp_port) looks like from
/// here, and is documented as looking like.
async fn mcp<C>(
    bytes: Bytes,
    channel: u32,
    port: u16,
    scope: Arc<ScopeHandle>,
    container: Arc<C>,
) where
    C: Container,
    C::Error: Into<Error>,
{
    relay(&bytes, channel, port, &scope, container.as_ref()).await;
    scope.send_channel_response_finish(channel).await;
}

/// The part of an MCP exchange that can give up part way through.
///
/// Split out so that the finish above happens whatever this does. There
/// is nowhere to report a failure on an MCP channel — its frames are a
/// head and a body, with no error among them — so what a caller sees is
/// the absence of a head.
async fn relay<C>(
    bytes: &Bytes,
    channel: u32,
    port: u16,
    scope: &ScopeHandle,
    container: &C,
) where
    C: Container,
    C::Error: Into<Error>,
{
    let Ok(ClientFrame::ChannelRequest { payload, .. }) =
        ClientFrame::decode(bytes)
    else {
        return;
    };
    let Ok(asked::Frame::Mcp(request)) = asked::Frame::decode(payload) else {
        return;
    };

    let body = request
        .body
        .map(|body| Bytes::copy_from_slice(body.get().as_bytes()))
        .unwrap_or_default();

    let mut builder = hyper::Request::builder()
        .method(match request.method {
            http::request::Method::Post => hyper::Method::POST,
            http::request::Method::Get => hyper::Method::GET,
            http::request::Method::Head => hyper::Method::HEAD,
            http::request::Method::Delete => hyper::Method::DELETE,
        })
        .uri(&request.path)
        // HTTP/1.1 requires one and the pipe makes it meaningless, so
        // it is stated rather than derived. A caller's own `Host`, if it
        // sent one, replaces this below.
        .header(hyper::header::HOST, "container");
    for (name, value) in &request.headers {
        builder = builder.header(name, value);
    }

    let Ok(built) = builder.body(Full::new(body)) else {
        return;
    };
    let Ok((reader, writer)) = container.connect(port).await else {
        return;
    };
    let Ok(answer) = crate::server::http::request(reader, writer, built).await
    else {
        return;
    };

    let head = http::response::Head {
        status: answer.status().as_u16(),
        headers: answer
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                let value = value.to_str().ok()?;
                Some((name.as_str().to_owned(), value.to_owned()))
            })
            .collect(),
    };

    let mut buffer = Vec::new();
    if channel_response::mcp::Frame::Head(head)
        .encode(&mut Writer::new(&mut buffer))
        .is_err()
    {
        return;
    }
    scope.send_channel_response(channel, &buffer).await;

    let mut body = BodyStream::new(answer.into_body());
    while let Some(Ok(frame)) = body.next().await {
        let Ok(piece) = frame.into_data() else {
            continue;
        };
        buffer.clear();
        if channel_response::mcp::Frame::Body(&piece)
            .encode(&mut Writer::new(&mut buffer))
            .is_ok()
        {
            scope.send_channel_response(channel, &buffer).await;
        }
    }
}

/// The plugin's half of a database connection, streamed to the caller.
///
/// The other half of the pair the provider opened. This one is the
/// caller's channel, and what it carries is everything the plugin
/// wrote — so the finish here says the plugin's socket ended, which is
/// what tells a caller to close its backend.
async fn writes(
    connection_id: u32,
    channel: u32,
    scope: Arc<ScopeHandle>,
    connections: Connections,
) {
    // An id nobody registered is a caller quoting one it was never
    // given, or one whose connection ended first. Either way there is
    // nothing to stream and the channel ends empty.
    let receiver = connections.lock().await.remove(&connection_id);

    if let Some(mut receiver) = receiver {
        let mut buffer = Vec::new();
        while let Some(bytes) = receiver.recv().await {
            buffer.clear();
            if channel_response::postgres::Frame(&bytes)
                .encode(&mut Writer::new(&mut buffer))
                .is_ok()
            {
                scope.send_channel_response(channel, &buffer).await;
            }
        }
    }

    scope.send_channel_response_finish(channel).await;
}

/// Take database connections as the plugin opens them.
///
/// See [`accept`] for the dialling, which is the whole mechanism. Each
/// pipe that turns out to be wanted becomes one connection: an id, a
/// channel opened outward for what the database says, and a place for
/// the caller to come and collect what the plugin says.
async fn postgres<C>(
    container: Arc<C>,
    port: u16,
    scope: Arc<ScopeHandle>,
    connections: Connections,
) where
    C: Container,
    C::Error: Into<Error>,
{
    // Counting up, and never reused while live: a wrapped counter would
    // need the set of open ids to step over, and a connection pool does
    // not reach four billion. One task mints them, so it is a number
    // rather than anything shared.
    let mut ids = 0u32;
    let mut taken = tokio::task::JoinSet::new();

    while let Some((first, reader, writer)) = accept(container.as_ref(), port).await {
        while taken.try_join_next().is_some() {}

        ids += 1;
        let connection_id = ids;
        let (sender, receiver) = mpsc::unbounded_channel();
        connections.lock().await.insert(connection_id, receiver);

        let mut payload = Vec::new();
        let postgres = channel_request::Postgres { connection_id };
        if channel_request::Frame::Postgres(postgres)
            .encode(&mut Writer::new(&mut payload))
            .is_err()
        {
            connections.lock().await.remove(&connection_id);
            continue;
        }
        // After the receiver is in place, so a caller cannot arrive
        // before the thing it is coming for.
        let channel = scope.send_channel_request(&payload).await;

        taken.spawn(connection(first, reader, writer, sender, channel));
    }
}

/// One database connection, pumped both ways.
///
/// Two directions and neither waits on the other: what the plugin
/// writes goes to the caller through the queue, and what the database
/// says comes back on the channel and goes into the pipe.
async fn connection<R, W>(
    first: Bytes,
    reader: R,
    writer: W,
    sender: mpsc::UnboundedSender<Bytes>,
    channel: Channel,
) where
    R: futures_util::Stream<Item: TryIntoBytes> + Send + Unpin + 'static,
    W: futures_util::Sink<Bytes> + Send + Unpin + 'static,
{
    let outward = async move {
        // The byte that said this pipe was wanted is part of the
        // conversation, not a signal outside it.
        if sender.send(first).is_err() {
            return;
        }
        let mut reader = reader;
        while let Some(item) = reader.next().await {
            let Some(bytes) = item.bytes() else { return };
            if sender.send(bytes).is_err() {
                return;
            }
        }
    };

    let inward = async move {
        let mut writer = writer;
        let mut channel = channel;
        while let Some(bytes) = channel.response_receiver.recv().await {
            let Ok(ClientFrame::ChannelResponse { payload, .. }) =
                ClientFrame::decode(&bytes)
            else {
                continue;
            };
            // Bytes are bytes; there is nothing here that can fail to
            // be read, and the type says so.
            let answered::postgres::Frame(piece) =
                answered::postgres::Frame::decode(payload)
                    .unwrap_or_else(|error| match error {});
            if writer.send(Bytes::copy_from_slice(piece)).await.is_err() {
                return;
            }
        }
    };

    futures_util::future::join(outward, inward).await;
}

/// Take commands as the plugin asks for them.
///
/// One pipe is one command. What the plugin writes after the id is the
/// ask, and the end of its writes is where the ask ends — see
/// [`Container::connect`] for why a half-close carries that rather than
/// a length in front of it.
async fn command<C>(container: Arc<C>, port: u16, scope: Arc<ScopeHandle>)
where
    C: Container,
    C::Error: Into<Error>,
{
    let mut asks = tokio::task::JoinSet::new();

    while let Some((first, reader, writer)) = accept(container.as_ref(), port).await {
        while asks.try_join_next().is_some() {}
        asks.spawn(ask(first, reader, writer, Arc::clone(&scope)));
    }
}

/// One command: read it whole, relay it, and stream the answers back.
async fn ask<R, W>(first: Bytes, reader: R, writer: W, scope: Arc<ScopeHandle>)
where
    R: futures_util::Stream<Item: TryIntoBytes> + Send + Unpin + 'static,
    W: futures_util::Sink<Bytes> + Send + Unpin + 'static,
{
    let mut buffer = BytesMut::from(&first[..]);
    let mut reader = reader;
    // To the end of the plugin's writes, which is the ask. Nothing
    // delimits it because nothing has to: this pipe carries one.
    while let Some(item) = reader.next().await {
        let Some(bytes) = item.bytes() else { break };
        buffer.extend_from_slice(&bytes);
    }

    // The exchange, fixed width and unchanging, and then the ask. It is
    // the plugin's to mint and the plugin's to keep unique among the
    // commands it has open — nothing here reads it twice.
    if buffer.len() < EXCHANGE_LEN {
        return;
    }
    let ask = buffer.split_off(EXCHANGE_LEN);

    let mut payload = Vec::new();
    if channel_request::Frame::Command(&ask)
        .encode(&mut Writer::new(&mut payload))
        .is_err()
    {
        return;
    }
    let mut channel = scope.send_channel_request(&payload).await;

    let mut writer = writer;
    while let Some(bytes) = channel.response_receiver.recv().await {
        let Ok(ClientFrame::ChannelResponse { payload, .. }) =
            ClientFrame::decode(&bytes)
        else {
            continue;
        };
        // An item is bytes and arrives as the same bytes, so reading
        // one has no failure mode.
        let answered::command::Frame(item) =
            answered::command::Frame::decode(payload)
                .unwrap_or_else(|error| match error {});
        if writer.send(Bytes::copy_from_slice(item)).await.is_err() {
            return;
        }
    }
    // The channel finished, so the command did. Dropping the writer
    // closes the pipe, which is how the plugin is told.
}

/// The bytes a command's exchange id occupies.
const EXCHANGE_LEN: usize = 4;

/// Keep one connection dialled, and hand it over when it is wanted.
///
/// A plugin listens and therefore cannot ask for a pipe, so the
/// provider keeps one waiting. The first byte is the signal that it has
/// been taken, and it is a sound one because both conduits exist for
/// the plugin to speak first: pgwire's startup message is the client's,
/// and a command conduit is a plugin asking.
///
/// # What it costs
///
/// One idle connection per conduit at a time, and a pool warming to N
/// takes N dials a round trip apart. Both are local, and neither is
/// worth a wire format to avoid.
///
/// # A pipe that dies unused is dialled again
///
/// Which is the one place this could spin. It cannot spin quickly: each
/// turn is a connect and a read that reached the end, so a container
/// closing pipes as fast as they open is a container that has stopped
/// working, and the stop that follows is what ends this.
async fn accept<C>(
    container: &C,
    port: u16,
) -> Option<(Bytes, C::Reader, C::Writer)>
where
    C: Container,
{
    loop {
        let (mut reader, writer) = container.connect(port).await.ok()?;
        match reader.next().await {
            Some(item) => {
                if let Some(bytes) = item.bytes() {
                    return Some((bytes, reader, writer));
                }
            }
            None => continue,
        }
    }
}

/// One item off a container's pipe, if it was bytes.
///
/// [`Container::Reader`](Container::Reader) yields
/// `Result<Bytes, C::Error>` and every consumer here wants the same
/// thing from it: the bytes, or an end. Naming that as a trait is what
/// lets the pumps be generic over a reader without carrying its error
/// type around to discard it.
trait TryIntoBytes {
    /// The bytes, or [`None`] if this item was a failure.
    fn bytes(self) -> Option<Bytes>;
}

impl<E> TryIntoBytes for Result<Bytes, E> {
    fn bytes(self) -> Option<Bytes> {
        self.ok()
    }
}

/// End the scope, which needs the handle back to itself.
///
/// [`send_response_finish`](ScopeHandle::send_response_finish) consumes
/// the handle, which is what makes one finish per scope a fact rather
/// than a rule — so it cannot be reached through a share, and this is
/// where the shares are proved gone.
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
