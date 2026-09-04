//! Starting a plugin, and answering everything it needs while it runs.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::StreamExt as _;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::super::channel_response::{command, postgres};
use super::execute_handle::ExecuteHandle;
use super::super::{channel_request, request};
use crate::client::command_proxy::CommandProxy;
use crate::client::handle::{Handle, SendError};
use crate::client::oci_proxy::OciProxy;
use crate::client::postgres_proxy::PostgresProxy;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::mcp_plugin::run::server::channel_request as server_channel_request;
use crate::shared::oci;
use crate::frame;

/// Run an MCP plugin, and serve it for as long as it lives.
///
/// # Three things at once, where a loop does one
///
/// An [`agentic loop`](crate::endpoints::agentic_loop::run::client::execute)
/// is asked for one thing while it runs, so it takes one proxy. A
/// plugin is asked for three, and they have nothing in common but the
/// scope they arrive in: the image it was built from, a database it
/// dialled, and commands it cannot run itself.
///
/// So this takes three proxies and puts one task on all of them. What
/// comes back is an [`ExecuteHandle`], which is nothing to poll — the
/// answering happens beside it, and a caller that never touches the
/// handle again still gets its image served.
///
/// # Which proxies are actually used
///
/// Whichever the provider asks for, and it may ask for none of them. A
/// plugin whose image the provider already holds opens no registry
/// channel; one that never dials its conduit opens no database channel.
/// All three are required anyway, because what a container does is not
/// knowable when it is started.
///
/// # One task per exchange, not one for all of them
///
/// The loop reading channel requests spawns rather than serving in
/// place, because every one of these can be long: a layer is hundreds
/// of megabytes, a command runs as long as it runs, and a database
/// connection lasts the plugin's life. Serving in place would put every
/// later request behind whichever one is longest-lived.
///
/// It is uniform across all three, which it could not have been before
/// a Postgres connection became a
/// [`pair of channels`](crate::endpoints::mcp_plugin::run::server::channel_request::Postgres).
/// When one channel carried a socket, its later frames had to find the
/// task already serving it, and that is a map this does not need: every
/// channel request now opens one exchange and is answered once.
///
/// # [`Arc`]s rather than values
///
/// So one set of proxies can serve several plugins. It is what a caller
/// has anyway: a registry and a database are things you stand up once,
/// and a plugin is a thing you run many times.
///
/// # It fails in only one way
///
/// The request either serializes or it does not, and then it either
/// goes out or it does not. Everything after that belongs to the run
/// rather than to the asking — a provider that cannot start the
/// container says so in a frame. See [`RunError`](super::RunError).
pub async fn execute<O, P, C>(
    handle: &Handle,
    request: &request::Frame,
    oci_proxy: Arc<O>,
    postgres_proxy: Arc<P>,
    command_proxy: Arc<C>,
) -> Result<ExecuteHandle, ExecuteError>
where
    O: OciProxy + 'static,
    P: PostgresProxy<request::Frame> + 'static,
    C: CommandProxy + 'static,
{
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;

    let scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    tokio::spawn(serve_channel_requests(
        scope.request_receiver,
        handle.clone(),
        scope.scope,
        Arc::new(request.clone()),
        oci_proxy,
        postgres_proxy,
        command_proxy,
    ));
    Ok(ExecuteHandle::new(
        scope.scope,
        scope.response_receiver,
        handle.clone(),
    ))
}

/// Take what the provider asks for, and put each ask on its own task.
///
/// Ends when the receiver closes, which is the scope ending — a
/// [`Router`](crate::client::router::Router) drops everything under a
/// finished scope, this receiver with it. Nothing else stops it, except
/// nothing, since the task is detached — it simply outlives a caller
/// that has stopped caring, until the scope it reads closes.
///
/// The frame is passed on whole and undecoded. Decoding it here would
/// mean either borrowing across the spawn, which cannot be done, or
/// copying out of it, which would be a copy per request for nothing.
///
/// The plugin's own request rides along in an [`Arc`] because
/// [`PostgresProxy::handle`] borrows it per connection, and it has to
/// outlive this call for that to be possible.
async fn serve_channel_requests<O, P, C>(
    mut request_receiver: UnboundedReceiver<Bytes>,
    handle: Handle,
    scope: u32,
    request: Arc<request::Frame>,
    oci_proxy: Arc<O>,
    postgres_proxy: Arc<P>,
    command_proxy: Arc<C>,
) where
    O: OciProxy + 'static,
    P: PostgresProxy<request::Frame> + 'static,
    C: CommandProxy + 'static,
{
    while let Some(bytes) = request_receiver.recv().await {
        tokio::spawn(serve_one(
            bytes,
            handle.clone(),
            scope,
            Arc::clone(&request),
            Arc::clone(&oci_proxy),
            Arc::clone(&postgres_proxy),
            Arc::clone(&command_proxy),
        ));
    }
}

/// Read one ask, and hand it to whichever proxy answers it.
///
/// # What a frame it cannot read does
///
/// Nothing, and the channel is left unfinished. None of these channels
/// has an error variant — each answers in a vocabulary that already
/// has one — but a request this end cannot decode has nothing to answer
/// IN, because which of the three it was is the thing that did not
/// parse. Finishing an unanswered channel would say the answer was
/// empty, which is a different and worse thing to say.
async fn serve_one<O, P, C>(
    bytes: Bytes,
    handle: Handle,
    scope: u32,
    request: Arc<request::Frame>,
    oci_proxy: Arc<O>,
    postgres_proxy: Arc<P>,
    command_proxy: Arc<C>,
) where
    O: OciProxy,
    P: PostgresProxy<request::Frame>,
    C: CommandProxy,
{
    let Ok(frame::server::ServerFrame::ChannelRequest {
        channel, payload, ..
    }) = frame::server::ServerFrame::decode(&bytes)
    else {
        return;
    };
    match server_channel_request::Frame::decode(payload) {
        Ok(server_channel_request::Frame::Oci(request)) => {
            // Refcounted rather than copied: the answer outlives this
            // frame by as long as a layer takes to send, so a borrow
            // could not have gone with it.
            let request = bytes.slice_ref(request.0);
            serve_oci(&handle, scope, channel, request, &*oci_proxy).await;
        }
        Ok(server_channel_request::Frame::Command(command)) => {
            // Refcounted rather than copied: the command outlives this
            // frame, so a borrow could not have gone with it.
            let command = bytes.slice_ref(command);
            serve_command(&handle, scope, channel, command, &*command_proxy)
                .await;
        }
        Ok(server_channel_request::Frame::Postgres(connection)) => {
            serve_postgres(
                &handle,
                scope,
                channel,
                connection.connection_id,
                &request,
                &*postgres_proxy,
            )
            .await;
        }
        // Same reasoning as the arms above, which all finish the
        // channel they served: one that cannot be served still has to
        // be ended, or a provider waits on it forever. The likely
        // cause is a provider newer than this client, sending a
        // variant that did not exist when it was built.
        Err(_) => {
            let _ = handle.send_channel_response_finish(scope, channel).await;
        }
    }
}

/// Answer one registry request.
///
/// Whatever the caller's registry says, in the pieces it says it in,
/// and then a finish. There is no head, because there is nothing to
/// read a status out of — a registry answer is bytes off a socket and
/// this end relays them without looking. See
/// [`oci::response::Frame`](crate::shared::oci::response::Frame).
///
/// # It stops at the first refusal
///
/// A [`Handle`] says whether a frame went out, and a failure means no
/// later one will either — the connection is gone, or the scope is. So
/// this returns rather than going on, which matters most for a layer: a
/// blob is the longest thing this protocol sends, and going on writing
/// one at a provider that has stopped listening is the most work
/// available to waste.
async fn serve_oci<O>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    request: Bytes,
    oci_proxy: &O,
) where
    O: OciProxy,
{
    let mut answer = oci_proxy.handle(request).await;
    let mut buffer = Vec::new();
    while let Some(piece) = answer.next().await {
        if !send_oci_body(handle, scope, channel, &mut buffer, &piece).await {
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
/// first refusal. A [`bool`] rather than the [`SendError`] itself,
/// because there is one thing to do about every one of them here and it
/// is stop. Encoding cannot fail — an answer is bytes and has nothing
/// to get wrong — so the only `false` is a send that did not land.
async fn send_oci_body(
    handle: &Handle,
    scope: u32,
    channel: u32,
    buffer: &mut Vec<u8>,
    body: &[u8],
) -> bool {
    buffer.clear();
    // The shared type rather than this endpoint's alias of it: an
    // alias names a tuple struct but cannot construct one.
    oci::response::Frame(body)
        .encode(&mut Writer::new(buffer))
        .unwrap_or_else(|error| match error {});
    handle
        .send_channel_response(scope, channel, buffer)
        .await
        .is_ok()
}

/// Run one command, and stream its items back.
///
/// One frame per item, then a finish. A failure is the last item and
/// travels as an
/// [`Error`](command::Frame::Error) frame, which is what makes it
/// something a plugin can act on rather than something it has to
/// recognise inside an item it was meant to pass along.
///
/// It stops at the first refusal, for the same reason a registry answer
/// does: a failure to send means the connection or the scope is gone,
/// and nothing later will land either.
async fn serve_command<C>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    command: Bytes,
    command_proxy: &C,
) where
    C: CommandProxy,
{
    let mut items = command_proxy.run(command).await;
    let mut buffer = Vec::new();
    while let Some(item) = items.next().await {
        buffer.clear();
        let frame = match &item {
            Ok(item) => command::Frame::Item(item),
            Err(error) => command::Frame::Error(error.clone()),
        };
        if frame.encode(&mut Writer::new(&mut buffer)).is_err() {
            break;
        }
        if handle
            .send_channel_response(scope, channel, &buffer)
            .await
            .is_err()
        {
            return;
        }
        // Nothing follows an error. The finish below is what says the
        // command is over, and it says it either way.
        if item.is_err() {
            break;
        }
    }
    let _ = handle.send_channel_response_finish(scope, channel).await;
}

/// Splice one database connection onto the pair of channels carrying
/// it.
///
/// The provider opened one half, asking for what the database says.
/// This opens the other, asking for what the plugin writes, and hands
/// both to a [`PostgresProxy`].
///
/// # The queue is made before anything is awaited
///
/// Which is the whole reason the proxy takes a receiver instead of
/// handing back a sink. The plugin wrote its startup message the
/// instant it connected and the provider has been holding it since, so
/// the writes start arriving the moment the second channel opens —
/// while the proxy is still dialling. They queue instead of waiting on
/// the dial, in the order they were written.
///
/// # What each finish means
///
/// The pump ending means the provider finished the channel this opened:
/// the plugin's socket is gone. Dropping the sender is how the proxy
/// hears it, and its answer is to hang up on the database and end the
/// stream.
///
/// This finishing the provider's channel means the database connection
/// is gone, and a provider answers it by shutting the plugin's socket.
/// Which is the half that could not be said at all until a connection
/// became two channels.
///
/// # A caller that cannot dial
///
/// Returns a stream that ends, so this finishes the provider's channel
/// having sent nothing. The provider closes the plugin's socket, the
/// plugin's driver reports a server that hung up on it — which is the
/// truth — and the channel this opened is finished by the provider on
/// its way out.
async fn serve_postgres<P>(
    handle: &Handle,
    scope: u32,
    server_channel: u32,
    connection_id: u32,
    request: &request::Frame,
    postgres_proxy: &P,
) where
    P: PostgresProxy<request::Frame>,
{
    // The encode cannot fail — a connection id is four known bytes —
    // but it shares an impl with a variant that can, so the failure is
    // treated as a refusal rather than unwrapped. Nothing is left half
    // open by returning here: the channel was never asked for.
    let mut buffer = Vec::new();
    if channel_request::Frame::Postgres(channel_request::Postgres {
        connection_id,
    })
    .encode(&mut Writer::new(&mut buffer))
    .is_err()
    {
        return;
    }
    let Ok(channel) = handle.send_channel_request(scope, &buffer).await else {
        return;
    };

    let (write_sender, write_receiver) = mpsc::unbounded_channel();
    tokio::spawn(pump_writes(channel.response_receiver, write_sender));

    let mut database = postgres_proxy.handle(request, write_receiver).await;
    while let Some(bytes) = database.next().await {
        buffer.clear();
        postgres::Frame(&bytes)
            .encode(&mut Writer::new(&mut buffer))
            .unwrap_or_else(|error| match error {});
        if handle
            .send_channel_response(scope, server_channel, &buffer)
            .await
            .is_err()
        {
            return;
        }
    }
    let _ = handle
        .send_channel_response_finish(scope, server_channel)
        .await;
}

/// Take the plugin's writes off the channel and hand on their payloads.
///
/// A [`Channel`](crate::client::channel::Channel) carries whole frames,
/// and a [`PostgresProxy`] takes the bytes the plugin wrote. This is
/// the strip in between — nine bytes of header off the front, and the
/// rest passed on unmodified, because anything else in front of a
/// startup message is garbage to the database.
///
/// The slice is refcounted rather than copied. It is a view of the
/// frame it arrived in, which stays alive exactly as long as the write
/// does.
///
/// # It is one task, and that is what preserves order
///
/// A pgwire client pipelines, and nothing in a pgwire message says
/// which request it answers — the server replies in order and a driver
/// matches by position. So these must reach the socket in the order
/// they were written, and one sequential task is what guarantees it. A
/// spawn per frame would be the same shape as the request dispatcher
/// above and would silently succeed against the wrong statements.
///
/// # Returning drops the sender, and that is the signal
///
/// The finish frame means the plugin's socket ended. Dropping the
/// sender turns that into the [`None`] a proxy reads as the plugin
/// having hung up — the one thing pgwire cannot say for a plugin that
/// crashed without sending `Terminate`.
async fn pump_writes(
    mut response_receiver: UnboundedReceiver<Bytes>,
    write_sender: UnboundedSender<Bytes>,
) {
    while let Some(bytes) = response_receiver.recv().await {
        let Ok(frame::server::ServerFrame::ChannelResponse {
            payload, ..
        }) = frame::server::ServerFrame::decode(&bytes)
        else {
            // The finish, or something that does not belong on this
            // channel. Either way there are no more writes.
            return;
        };
        let postgres::Frame(write) = postgres::Frame::decode(payload)
            .unwrap_or_else(|error| match error {});
        if write_sender.send(bytes.slice_ref(write)).is_err() {
            return;
        }
    }
}

/// A plugin that never started.
///
/// Everything a provider might object to is objected to afterwards, in
/// a frame — see [`RunError`](super::RunError), which is the run that
/// started and then stopped without ending.
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
                write!(f, "mcp plugin request did not serialize: {error}")
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
