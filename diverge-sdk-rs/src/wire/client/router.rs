//! The read loop: frames in, and off to whoever is waiting.

use std::collections::HashMap;
use std::fmt;
use std::pin::pin;

use futures_util::StreamExt as _;
use futures_util::future::{self, Either};
use futures_util::stream::SplitStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::handle::WeakHandle;
use super::registration::Registration;
use crate::wire::connection::Connection;
use crate::wire::credit::{self, Credit, Ended, Filer, Kind, Window};
use crate::wire::frame::server::ServerFrame;

/// One connection's inbound half.
///
/// Reads frames, looks up where each belongs, and forwards it. It is
/// the only thing that touches the read half of a
/// [`Connection`], and nothing outside this crate is expected to hold
/// one — what a caller touches is a handle that talks TO this.
///
/// # Whole frames, untouched
///
/// A frame goes on exactly as it came off the socket, header included.
/// The header is read to know where the frame belongs and then left
/// alone, because the far side needs it too: a finish is a frame, and
/// telling one from a response means reading the type. Forwarding
/// costs a refcount bump.
///
/// Bytes rather than a decoded frame, and that is forced rather than
/// chosen — a decoded
/// [`ClientFrame`](crate::wire::frame::client::ClientFrame) borrows from the
/// buffer it came from, so it cannot cross a channel at all.
///
/// # One hop
///
/// A frame goes straight from here to its consumer. Routing by scope
/// to a per-scope task that then routes by channel would buy a second
/// wakeup and a second scheduler dispatch on every frame — a map
/// lookup is tens of nanoseconds and a task hop is hundreds to
/// thousands.
///
/// # Nothing here waits
///
/// Filing a frame is a push into a queue that always has room for it,
/// because the server sends a frame only into room this end's reader
/// gave it (see [`credit`]). So this loop never blocks on a consumer,
/// and one scope that has stopped reading cannot stall another or the
/// socket: it stalls its own sender instead, on the far side. A frame
/// that finds no room broke the rule, and ends the connection.
///
/// The credits this end owes go out beside the loop, never from inside
/// it: a read loop waiting on the socket's write half could wait on a
/// far end that is itself waiting to be read.
///
/// # When entries leave
///
/// A finish takes one out, which is the only removal there is, and a
/// closure goes out to whoever is keeping a matching record. A consumer
/// that walked away without a finish told the server so, and a frame
/// still on its way to it is dropped.
#[derive(Debug)]
pub struct Router {
    /// The read half of the connection.
    stream: SplitStream<Connection>,
    /// Every scope open on this connection.
    ///
    /// Nested rather than flattened into a `(scope, channel)` key,
    /// because a scope ending ends everything under it: one removal
    /// drops every queue and ends every window at once.
    scopes: HashMap<u32, Entry>,
    /// Somewhere to register a new destination before its frames
    /// arrive. See [`Registration`].
    ///
    /// Unbounded on purpose. Registering must not block, because
    /// whoever is registering is about to write the request that
    /// causes the frames, and it is drained from inside the loop that
    /// fills the rest.
    registration_receiver: UnboundedReceiver<Registration>,
    /// Somewhere to say an entry is gone: `(scope, channel)`, `None`
    /// for the scope itself.
    closed_sender: UnboundedSender<(u32, Option<u32>)>,
    /// Where the window of each channel the server opens goes, so the
    /// handle answering it can wait on it: `(scope, channel, window)`.
    windows_sender: UnboundedSender<(u32, u32, Window)>,
    /// The credits this end's readers owe, and the handle that sends
    /// them; taken by [`run`](Self::run).
    outbox: Option<(UnboundedReceiver<Credit>, WeakHandle)>,
}

/// One open scope.
#[derive(Debug)]
struct Entry {
    /// Where the answers on channel `0` go.
    responses: Filer,
    /// Where the server's channel requests go.
    requests: Filer,
    /// The channels this client opened, and where each one's answer goes.
    channels: HashMap<u32, Filer>,
    /// This end's room for channel requests in the scope.
    asked: Window,
    /// This end's room on each channel the server opened, by the
    /// server's number.
    answers: HashMap<u32, Window>,
}

impl Entry {
    /// Rule 7 of [`credit`], for everything under the scope.
    fn end(&self, why: Ended) {
        self.asked.end(why);
        for window in self.answers.values() {
            window.end(why);
        }
    }
}

impl Router {
    /// Take the parts a router is made of — the complements of what
    /// [`Handle::open`](super::handle::Handle::open) gives its handle.
    pub(super) fn new(
        stream: SplitStream<Connection>,
        registration_receiver: UnboundedReceiver<Registration>,
        closed_sender: UnboundedSender<(u32, Option<u32>)>,
        windows_sender: UnboundedSender<(u32, u32, Window)>,
        outbox: UnboundedReceiver<Credit>,
        handle: WeakHandle,
    ) -> Self {
        Router { stream, scopes: HashMap::new(), registration_receiver, closed_sender, windows_sender, outbox: Some((outbox, handle)) }
    }

    /// Read frames until the connection ends, sending the credits this
    /// end owes beside them.
    ///
    /// Returns when the stream ends — or, early, when a token arrives,
    /// since each end sends ONE in the [`handshake`](crate::wire::handshake)
    /// that was done before this router existed; or when a frame breaks
    /// rule 6 of [`credit`]. Either way this is dropped, and with it every
    /// queue in it: a consumer whose receiver ends without a finish lost
    /// the connection mid-stream. Every window ends with it (rule 7).
    ///
    /// A transport error is not the end. It yields no frame, so there is
    /// nothing to route and the loop takes the next one — which is the
    /// end, in practice, because both transports fuse.
    ///
    /// # Discarding
    ///
    /// A frame nobody is waiting for is dropped, silently: a header too
    /// short to read, a type this layer does not define, a scope with no
    /// entry, a channel with no entry, a reader that has gone.
    pub async fn run(mut self) -> Result<(), InvalidAuthorize> {
        let (outbox, handle) = self.outbox.take().expect("a router runs once");
        let read = pin!(self.read());
        let pump = pin!(pump(outbox, handle));
        match future::select(read, pump).await {
            Either::Left((ended, _)) => ended,
            // The pump ends when the handle is gone; reading goes on.
            Either::Right(((), read)) => read.await,
        }
    }

    /// The loop itself, ending every window when it ends.
    async fn read(&mut self) -> Result<(), InvalidAuthorize> {
        let ended = self.route().await;
        for entry in self.scopes.values() {
            entry.end(Ended::Connection);
        }
        ended
    }

    async fn route(&mut self) -> Result<(), InvalidAuthorize> {
        while let Some(received) = self.stream.next().await {
            let Ok(bytes) = received else { continue };
            // Decoded for its header alone; what is forwarded is
            // `bytes`, whole and untouched.
            let Ok(frame) = ServerFrame::decode(&bytes) else { continue };
            let filed = match frame {
                ServerFrame::Auth { .. } => return Err(InvalidAuthorize),
                ServerFrame::Response { scope, .. } => match self.scope(scope) {
                    Some(entry) => credit::file(&entry.responses, bytes),
                    None => Ok(()),
                },
                // The scope is over, so everything under it goes. Filed
                // first, because the finish is what says the stream
                // ended rather than the connection.
                ServerFrame::ResponseFinish { scope } => {
                    let filed = match self.scope(scope) {
                        Some(entry) => credit::file(&entry.responses, bytes),
                        None => Ok(()),
                    };
                    self.close_scope(scope);
                    filed
                }
                // The scope's queue, not a channel's: the number is the
                // SERVER's. Its window is made here and announced before
                // the request is filed, so whoever answers finds it.
                ServerFrame::ChannelRequest { scope, channel, .. } => {
                    if !self.scopes.contains_key(&scope) {
                        self.drain();
                    }
                    match self.scopes.get_mut(&scope) {
                        Some(entry) => {
                            let window = Window::new();
                            entry.answers.insert(channel, window.clone());
                            let _ = self.windows_sender.send((scope, channel, window));
                            credit::file(&entry.requests, bytes)
                        }
                        None => Ok(()),
                    }
                }
                ServerFrame::ChannelResponse { scope, channel, .. } => match self.channel(scope, channel) {
                    Some(filer) => credit::file(filer, bytes),
                    None => Ok(()),
                },
                ServerFrame::ChannelResponseFinish { scope, channel } => {
                    let filed = match self.channel(scope, channel) {
                        Some(filer) => credit::file(filer, bytes),
                        None => Ok(()),
                    };
                    self.close_channel(scope, channel);
                    filed
                }
                // Room on a stream this end sends: the scope's channel
                // requests, or its answer on one of the server's
                // channels. An unknown stream or kind is nothing.
                ServerFrame::Credit { scope, channel, kind } => {
                    if let (Some(kind), Some(entry)) = (Kind::of(kind), self.scope(scope)) {
                        match kind {
                            Kind::Requests | Kind::RequestsAbandoned => entry.asked.apply(kind),
                            Kind::Responses | Kind::ResponsesAbandoned => {
                                if let Some(window) = entry.answers.get(&channel) {
                                    window.apply(kind);
                                }
                            }
                        }
                    }
                    Ok(())
                }
            };
            if filed.is_err() {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Find a scope, draining registrations first if it is not there —
    /// a registration and the frames it is for travel by different
    /// routes, and the frames can win.
    fn scope(&mut self, scope: u32) -> Option<&mut Entry> {
        if !self.scopes.contains_key(&scope) {
            self.drain();
        }
        self.scopes.get_mut(&scope)
    }

    /// Find a channel this client opened, on the same terms.
    fn channel(&mut self, scope: u32, channel: u32) -> Option<&Filer> {
        let found = self.scopes.get(&scope).is_some_and(|entry| entry.channels.contains_key(&channel));
        if !found {
            self.drain();
        }
        self.scopes.get(&scope)?.channels.get(&channel)
    }

    /// Drop a scope and everything under it, and end its windows.
    fn close_scope(&mut self, scope: u32) {
        if let Some(entry) = self.scopes.remove(&scope) {
            entry.end(Ended::Scope);
            let _ = self.closed_sender.send((scope, None));
        }
    }

    /// Drop a channel this client opened, and leave its scope alone.
    fn close_channel(&mut self, scope: u32, channel: u32) {
        let Some(entry) = self.scopes.get_mut(&scope) else { return };
        if entry.channels.remove(&channel).is_some() {
            let _ = self.closed_sender.send((scope, Some(channel)));
        }
    }

    /// Take every registration that is waiting. A duplicate scope or
    /// channel keeps the entry it found; a channel with no scope is
    /// dropped; an answered channel's window goes.
    fn drain(&mut self) {
        while let Ok(registration) = self.registration_receiver.try_recv() {
            match registration {
                Registration::Scope { scope, response_filer, request_filer, asked } => {
                    self.scopes.entry(scope).or_insert_with(|| Entry {
                        responses: response_filer,
                        requests: request_filer,
                        channels: HashMap::new(),
                        asked,
                        answers: HashMap::new(),
                    });
                }
                Registration::Channel { scope, channel, response_filer } => {
                    if let Some(entry) = self.scopes.get_mut(&scope) {
                        entry.channels.entry(channel).or_insert(response_filer);
                    }
                }
                Registration::Answered { scope, channel } => {
                    if let Some(entry) = self.scopes.get_mut(&scope) {
                        entry.answers.remove(&channel);
                    }
                }
            }
        }
    }
}

/// Send every credit this end's readers owe, until none can owe any
/// more or the handle is gone.
async fn pump(mut outbox: UnboundedReceiver<Credit>, handle: WeakHandle) {
    while let Some(credit) = outbox.recv().await {
        let Some(handle) = handle.upgrade() else { return };
        if handle.send_credit(credit).await.is_err() {
            return;
        }
    }
}

/// A token arrived after the handshake.
///
/// What [`Router::run`] returns instead of ending cleanly: each end
/// sends one token, in the [`handshake`](crate::wire::handshake) that
/// was done before this router existed. A far end presenting another
/// is disagreeing about what the handshake is, and routing stopped
/// where the disagreement started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvalidAuthorize;

impl fmt::Display for InvalidAuthorize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a token arrived after the handshake")
    }
}

impl std::error::Error for InvalidAuthorize {}

