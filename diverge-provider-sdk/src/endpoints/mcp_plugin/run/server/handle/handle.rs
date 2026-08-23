//! Running a plugin, and serving everything either end asks of it.

use std::collections::HashMap;
use std::pin::pin;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::future;
use futures_util::{Sink, SinkExt as _, Stream, StreamExt as _};
use indexmap::IndexMap;
use rmcp::model::{
    CallToolRequestParams, PaginatedRequestParams, ReadResourceRequestParams,
};
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
/// as environment, and a plugin reads it to decide how to behave. This
/// is what the PROVIDER established about the connection, and it goes
/// to the [`ContainerDeployer`], which is the party that decides
/// whether this caller may have a container at all.
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
) where
    D: ContainerDeployer,
    D::Error: Into<Error>,
{
    // Shared from here, because the workers write on it and none of
    // them may hold it alone. What stays exclusive is ENDING the scope,
    // which is why the finish has to get the handle back out.
    let scope = Arc::new(scope);

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
        match deploy(
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
                // This frame's error is `Infallible` and a
                // `ClientRegistry` wants the JSON one, because it also
                // serves a laboratory whose frame really can fail. An
                // empty match on an `Infallible` produces whatever is
                // asked for, there being no value to produce it from.
                channel_request::Frame::Oci(request)
                    .encode(out)
                    .map_err(|error| match error {})
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
/// channel the instant the plugin opens a connection, and the caller's
/// channel quoting that id turns up whenever the caller gets to it. So
/// the plugin's writes wait here in between.
///
/// The receiver is put in BEFORE the channel request that names the id
/// goes out, which is what makes the lookup safe: a caller cannot ask
/// about an id it has not been told, and by the time it has been told,
/// this holds it.
type Connections = Arc<Mutex<HashMap<u32, mpsc::UnboundedReceiver<Bytes>>>>;

/// Serve the plugin until it is stopped or the caller leaves.
///
/// Two conduits standing by for what the container asks — its database
/// connections and its commands — a task per exchange, and this reading
/// what the caller opens and handing each one on.
async fn serve<C>(
    scope: &Arc<ScopeHandle>,
    container: &Arc<C>,
    request: &request::Frame,
) where
    C: Container + 'static,
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
        // Finished ones, so the set does not grow for the life of the
        // plugin. An exchange is a task, and a plugin serving tools for
        // hours has a great many of them.
        while workers.try_join_next().is_some() {}

        // The caller is gone, and nothing it asked for matters now.
        let Some(bytes) = scope.recv_channel_request().await else {
            break;
        };

        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) =
            ClientFrame::decode(&bytes)
        else {
            continue;
        };

        // The MCP variants carry owned params, so each ask moves into
        // its task whole — nothing borrows the frame it arrived in, and
        // nothing has to be decoded twice.
        match asked::Frame::decode(payload) {
            Ok(asked::Frame::McpListTools(request)) => {
                workers.spawn(mcp_list_tools(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    mcp_port,
                    request.0,
                ));
            }
            Ok(asked::Frame::McpListResources(request)) => {
                workers.spawn(mcp_list_resources(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    mcp_port,
                    request.0,
                ));
            }
            Ok(asked::Frame::McpCallTool(request)) => {
                workers.spawn(mcp_call_tool(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    mcp_port,
                    request.0,
                ));
            }
            Ok(asked::Frame::McpReadResource(request)) => {
                workers.spawn(mcp_read_resource(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    mcp_port,
                    request.0,
                ));
            }
            Ok(asked::Frame::McpNotifications(_)) => {
                workers.spawn(mcp_notifications(
                    Arc::clone(scope),
                    channel,
                    Arc::clone(container),
                    mcp_port,
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

/// Answer one caller-opened channel, and finish it.
///
/// The unary MCP shape, written once: at most one frame, then the
/// finish, on every path.
///
/// [`None`] is the container not being reachable — nothing listening on
/// the MCP port, or a port that was never declared — and it sends
/// nothing at all. That is not this function inventing a shape: a
/// finish with nothing before it is already what the wire means by "the
/// provider could not serve the exchange", and it is exactly what
/// [`mcp_port`](request::Frame::mcp_port) documents a wrong port as
/// looking like. The caller's own error type names it `Unanswered`.
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

/// Ask the plugin what tools it has, for the caller.
///
/// The plugin's own MCP server answers, through
/// [`Container::mcp_list_tools`]. Its refusal is an answer — the
/// `Error` frame — where the container being unreachable is not, and
/// [`answer`] says what each becomes on the wire.
///
/// The three siblings below are this exchange against a different noun,
/// and differ in nothing else.
async fn mcp_list_tools<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    port: u16,
    params: Option<PaginatedRequestParams>,
) where
    C: Container,
{
    let frame = match container.mcp_list_tools(port, params).await {
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

/// Ask the plugin what resources it has, for the caller.
async fn mcp_list_resources<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    port: u16,
    params: Option<PaginatedRequestParams>,
) where
    C: Container,
{
    let frame = match container.mcp_list_resources(port, params).await {
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

/// Run one of the plugin's tools, for the caller.
async fn mcp_call_tool<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    port: u16,
    params: CallToolRequestParams,
) where
    C: Container,
{
    let frame = match container.mcp_call_tool(port, params).await {
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

/// Read one of the plugin's resources, for the caller.
async fn mcp_read_resource<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    port: u16,
    params: ReadResourceRequestParams,
) where
    C: Container,
{
    let frame = match container.mcp_read_resource(port, params).await {
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

/// Relay what the plugin says on its own account, for as long as it
/// says anything.
///
/// The one MCP exchange that is not answered once. The channel stays
/// open and every frame on it is another notification, until the
/// plugin's stream ends — or says why it will push no more, which is
/// the `Error` frame and is terminal by that frame's own contract.
///
/// A container that cannot be reached is a bare finish, same as the
/// unary four and for the same documented reason.
///
/// # The channel is finished on every path
///
/// Including after the error frame. The error says why the
/// notifications stopped; the finish says the channel is over, and they
/// are different facts on this wire.
async fn mcp_notifications<C>(
    scope: Arc<ScopeHandle>,
    channel: u32,
    container: Arc<C>,
    port: u16,
) where
    C: Container,
{
    let mut notifications = match container.mcp_notifications(port).await {
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
        // stream goes on: it is one thing the plugin said, and the next
        // may be fine.
        if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
            scope.send_channel_response(channel, &buffer).await;
        }
        if last {
            break;
        }
    }
    scope.send_channel_response_finish(channel).await;
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
            // Bytes are bytes: copying them into a frame has no failure
            // mode, and the type says so.
            channel_response::postgres::Frame(&bytes)
                .encode(&mut Writer::new(&mut buffer))
                .unwrap_or_else(|error| match error {});
            scope.send_channel_response(channel, &buffer).await;
        }
    }

    scope.send_channel_response_finish(channel).await;
}

/// Take database connections as the plugin opens them.
///
/// [`Container::postgres_serve`] yields one item per connection, when
/// the plugin opens one — the item's existence is the fact, so there is
/// nothing here to dial and nothing to probe. Each becomes one
/// connection: an id, a channel opened outward for what the database
/// says, and a place for the caller to come and collect what the plugin
/// says.
///
/// A container with nothing listening on the port never serves, and
/// this returns: the conduit never starts, and the plugin's pool never
/// fills — which is what
/// [`postgres_port`](request::Frame::postgres_port) documents a wrong
/// port as looking like.
async fn postgres<C>(
    container: Arc<C>,
    port: u16,
    scope: Arc<ScopeHandle>,
    connections: Connections,
) where
    C: Container,
{
    let Ok(mut opened) = container.postgres_serve(port).await else {
        return;
    };

    // Counting up, and never reused while live: a wrapped counter would
    // need the set of open ids to step over, and a connection pool does
    // not reach four billion. One task mints them, so it is a number
    // rather than anything shared.
    let mut ids = 0u32;
    let mut taken = tokio::task::JoinSet::new();

    while let Some((reader, writer)) = opened.next().await {
        while taken.try_join_next().is_some() {}

        ids += 1;
        let connection_id = ids;
        let (sender, receiver) = mpsc::unbounded_channel();
        // Before the channel request that names the id goes out, so a
        // caller cannot arrive before the thing it is coming for.
        connections.lock().await.insert(connection_id, receiver);

        let mut payload = Vec::new();
        let postgres = channel_request::Postgres { connection_id };
        channel_request::Frame::Postgres(postgres)
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|error| match error {});
        let channel = scope.send_channel_request(&payload).await;

        taken.spawn(connection(reader, writer, sender, channel));
    }
}

/// One database connection, pumped both ways.
///
/// Two directions and neither waits on the other: what the plugin
/// writes goes to the caller through the queue, and what the database
/// says comes back on the channel and goes into the pipe.
async fn connection<R, W, E>(
    reader: R,
    writer: W,
    sender: mpsc::UnboundedSender<Bytes>,
    channel: Channel,
) where
    R: Stream<Item = Result<Bytes, E>> + Send + Unpin + 'static,
    W: Sink<Bytes> + Send + Unpin + 'static,
{
    let outward = async move {
        let mut reader = reader;
        // An `Err` item is this connection's own failure, and it ends
        // it — the trait says a failure on one connection arrives in
        // its own reader, and this is the arriving.
        while let Some(Ok(bytes)) = reader.next().await {
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

    // Whichever ends first ends the connection. A join would have
    // waited for both, and only one of them has a reason to stop on its
    // own: the plugin closing its socket ends `outward`, while `inward`
    // waits on a channel the caller may never finish — so a dead
    // connection would have left a task alive for the plugin's life.
    //
    // Dropping them is also what closes the pipe, which is how the
    // plugin learns the other side went.
    let outward = pin!(outward);
    let inward = pin!(inward);
    future::select(outward, inward).await;
}

/// Take commands as the plugin asks for them.
///
/// [`Container::command_serve`] yields one item per command: the whole
/// ask, already read to the plugin's half-close, and the writer its
/// answers go back into. Each becomes one task, because a command runs
/// as long as it runs and the next one must not wait behind it.
///
/// A container with nothing listening never serves, and this returns —
/// the same silence a wrong
/// [`command_port`](request::Frame::command_port) is documented to be.
async fn command<C>(container: Arc<C>, port: u16, scope: Arc<ScopeHandle>)
where
    C: Container + 'static,
{
    let Ok(mut asks) = container.command_serve(port).await else {
        return;
    };

    let mut running = tokio::task::JoinSet::new();
    while let Some((ask, writer)) = asks.next().await {
        while running.try_join_next().is_some() {}
        running.spawn(run(ask, writer, Arc::clone(&scope)));
    }
}

/// One command: relay the ask out, and stream the answers back.
///
/// The channel finishing is the command being over, and dropping the
/// writer — which every path out of here does — is what closes the
/// plugin's pipe and says so.
///
/// # An error ends it the same way
///
/// [`Error`](answered::command::Frame::Error) is the caller saying the
/// command did not finish, and nothing follows it. The pipe closing is
/// the whole vocabulary the plugin has, so early is how it hears the
/// difference: what arrived is what the command produced, and the close
/// came before the caller's channel did.
async fn run<W>(ask: Bytes, writer: W, scope: Arc<ScopeHandle>)
where
    W: Sink<Bytes> + Send + Unpin + 'static,
{
    let mut payload = Vec::new();
    // Bytes copied into a frame: no failure mode, and the type says so.
    channel_request::Frame::Command(&ask)
        .encode(&mut Writer::new(&mut payload))
        .unwrap_or_else(|error| match error {});
    let mut channel = scope.send_channel_request(&payload).await;

    let mut writer = writer;
    while let Some(bytes) = channel.response_receiver.recv().await {
        let Ok(ClientFrame::ChannelResponse { payload, .. }) =
            ClientFrame::decode(&bytes)
        else {
            // The finish, or a frame with no business here. The first
            // ends the channel on its own and the second is not an
            // item, so neither is anything to write.
            continue;
        };
        match answered::command::Frame::decode(payload) {
            Ok(answered::command::Frame::Item(item)) => {
                if writer
                    .send(Bytes::copy_from_slice(item))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            // The command did not finish, and nothing follows. Closing
            // the pipe now is what tells the plugin.
            Ok(answered::command::Frame::Error(_)) => return,
            // An item this crate cannot read is an item the plugin
            // will not get, and relaying around the hole would hand the
            // plugin output with a silent gap in it. Truncation is at
            // least visible.
            Err(_) => return,
        }
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
