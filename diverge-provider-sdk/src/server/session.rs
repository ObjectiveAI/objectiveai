//! One connection, as the scopes a client opens on it.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Stream, StreamExt as _};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::scope_handle::ScopeHandle;
use crate::connection::Connection;
use crate::frame::client::ClientFrame;

/// A connection's provider side: the scopes a client opens on it.
///
/// A [`Stream`] of [`ScopeHandle`]s, and that is the whole of a
/// provider's outer loop — take a scope, spawn something to serve it,
/// take the next.
///
/// # Why this is not a router
///
/// The caller half has one, and it earns its keep there: a client
/// arranges where a scope's frames will go BEFORE opening it, so
/// something has to hold those arrangements and match arriving frames
/// against them. That is routing, and it is a job.
///
/// Nothing here can arrange anything. A server does not open scopes, it
/// is told about them — so there is no registration to match, and what
/// would have been a router is a loop that reads frames and hands out
/// the ones that open something.
///
/// # Polling this is what runs the connection
///
/// Not merely what accepts from it. Every frame on the socket comes
/// through [`poll_next`](Stream::poll_next), including the ones bound
/// for scopes already being served elsewhere — so a caller that stops
/// polling stops the connection, and a scope waiting on a channel in
/// some other task waits forever while the frames that would feed it
/// sit unread.
///
/// Which makes one rule absolute rather than advisory: **serve a scope
/// in its own task, never inside the loop that accepted it.** Doing the
/// work inline means nothing is reading the socket while it runs, and
/// if that work is itself waiting on the client, neither side moves
/// again.
///
/// # What is not here yet
///
/// **Channels this end opens.** A provider needs them — serving an
/// image, reaching a database, running a command — and there is no way
/// to open one. Which is why a channel response arriving now is
/// discarded: nothing here could have asked for it.
///
/// **Auth.** A credential belongs to the connection and there is
/// nowhere for one to go, in either direction. An
/// [`Auth`](ClientFrame::Auth) frame is discarded, and a provider on an
/// [`Outgoing`](crate::connection::Connection::Outgoing) connection
/// cannot send the one it owes.
#[derive(Debug)]
pub struct Session {
    /// The read half of the connection.
    ///
    /// Split from the write half at construction, because the two are
    /// used at once and from different places: this reads in a loop
    /// while the scopes it handed out write — see
    /// [`connection`](crate::connection) for why a
    /// [`Sink`](futures_util::Sink) needing `&mut` makes that a wire
    /// requirement rather than an implementation detail.
    stream: SplitStream<Connection>,
    /// The write half, shared by every scope on this connection.
    ///
    /// One socket, so one of these however many scopes are open, and a
    /// lock around it because a WebSocket forbids interleaving the
    /// fragments of two messages. One writer at a time is the wire's
    /// rule, not a queue discipline anybody chose.
    ///
    /// The lock is [`tokio`]'s, because the guard is held across the
    /// `await` that writes and a [`std`] guard is not [`Send`] across
    /// one.
    ///
    /// This end never writes. It holds this only to clone into the
    /// scopes it yields, which is the whole reason it takes a
    /// [`Connection`] rather than a [`SplitStream`] — the two halves
    /// have to start together for the scopes to get theirs.
    sink: Arc<Mutex<SplitSink<Connection, Bytes>>>,
    /// The scopes a client has open, and where what arrives inside each
    /// one goes.
    ///
    /// An address book rather than bookkeeping. It holds no counter and
    /// frees no numbers, because a server mints neither scopes nor
    /// anything in them: the numbers are the client's, and this end
    /// only ever looks them up.
    ///
    /// A plain [`HashMap`], single-owned, because every read and every
    /// write of it happens in [`poll_next`](Stream::poll_next). A
    /// shared map would put a lock on the lookup that runs per channel
    /// request, to save one on the removal that runs per scope.
    ///
    /// # Unbounded, and what that trades
    ///
    /// So that delivering a frame is a synchronous send that cannot
    /// wait. [`poll_next`](Stream::poll_next) cannot wait either, and a
    /// bounded queue would mean carrying a half-delivered frame across
    /// polls — a stored reservation, an allocation, and a stall that
    /// stops the whole connection whenever any one scope falls behind.
    ///
    /// What makes it defensible is what rides here. A channel request
    /// is ONE frame — the answer travels the other way, on channels of
    /// its own — so this queue holds channels a client has opened and
    /// the scope has not yet taken, not a byte stream. A scope that is
    /// being served at all drains it.
    ///
    /// What it costs is a bound. Those frames carry whole MCP requests,
    /// and a scope that never reads its own inbox lets a peer grow this
    /// without limit. Nothing here stops that, and nothing here can:
    /// the only remedy is to serve a scope or drop it, and dropping it
    /// frees the queue with it.
    scopes: HashMap<u32, UnboundedSender<Bytes>>,
    /// Where a scope says its handle is gone.
    ///
    /// Sent from [`ScopeHandle`]'s [`Drop`], so it cannot be forgotten
    /// and covers abandonment as well as any deliberate ending. See
    /// [`drain_closed`](Self::drain_closed) for why the notice is a
    /// prompt to look rather than an instruction to remove.
    ///
    /// Unbounded, because it is sent from a destructor, where there is
    /// nothing to await on and nowhere to report a failure.
    closed: UnboundedReceiver<u32>,
    /// The other end of it, kept to clone into every scope.
    closed_sender: UnboundedSender<u32>,
}

impl Session {
    /// Take a connection, however it was made.
    ///
    /// Splits it, and keeps both halves: the read half to run the loop,
    /// the write half to share out. Nothing else has to be supplied and
    /// nothing has to be paired up correctly, which is the difference
    /// between this and a design where the halves start apart.
    ///
    /// It neither dials nor accepts — see [`Connection`] — so what the
    /// URL is, what the TLS story is, and what authenticated the
    /// upgrade are all settled before this is called.
    pub fn new(connection: Connection) -> Self {
        let (sink, stream) = connection.split();
        let (closed_sender, closed) = mpsc::unbounded_channel();
        Session {
            stream,
            sink: Arc::new(Mutex::new(sink)),
            scopes: HashMap::new(),
            closed,
            closed_sender,
        }
    }

    /// Make a scope's inbox, and hand it out with the request that
    /// opened it.
    ///
    /// [`None`] for a scope that is already open, which is the whole of
    /// what can go wrong here. A scope this end did not create, holding
    /// a number this end does not mint, needs nothing decided about it
    /// beyond somewhere to put what arrives inside.
    ///
    /// The receiver goes out with the request rather than after it,
    /// because half a scope is not a thing a consumer could do anything
    /// with — and because it has to reach the same party that got the
    /// request, which handing it over separately could not guarantee.
    ///
    /// # Occupied is not the same as open
    ///
    /// An entry whose sender reports
    /// [`is_closed`](UnboundedSender::is_closed) is a scope whose
    /// handle is gone, and it is overwritten rather than refused. A
    /// client is free to reuse a scope number once that scope has
    /// ended, and refusing on the strength of an entry nobody holds
    /// would refuse a legitimate request over bookkeeping that has not
    /// caught up.
    ///
    /// # Why the sweep lives here
    ///
    /// Because this is the only place the map can GROW, and the drain
    /// is housekeeping rather than correctness — the check above reads
    /// the sender's own state, so a stale entry is already treated as
    /// no entry whether or not it has been swept.
    ///
    /// Which leaves the question of how often, and the answer is: as
    /// rarely as possible. Draining once per poll would pay for every
    /// spurious wakeup; once per frame would pay for every channel
    /// request. Here it is paid once per scope opened, which is exactly
    /// the rate at which the thing it cleans accumulates.
    fn open_scope(
        &mut self,
        scope: u32,
        request: Bytes,
    ) -> Option<ScopeHandle> {
        self.drain_closed();
        if self.scopes.get(&scope).is_some_and(|sender| !sender.is_closed())
        {
            return None;
        }
        let (sender, receiver) = mpsc::unbounded_channel();
        self.scopes.insert(scope, sender);
        Some(ScopeHandle::new(
            scope,
            request,
            receiver,
            self.closed_sender.clone(),
            self.sink.clone(),
        ))
    }

    /// Forget every scope whose handle has been dropped.
    ///
    /// Run from [`open_scope`](Self::open_scope) and nowhere else — see
    /// there for why that is the one moment worth paying for. Between
    /// two scopes opening, dead entries sit in the map and their
    /// notices sit in the queue, and neither costs anything but the
    /// room: a lookup is no slower for them, and the check that matters
    /// does not consult them.
    ///
    /// Both are bounded by the same thing. A notice exists because a
    /// scope existed, and every scope that opens drains all of them, so
    /// what accumulates between drains cannot exceed what has been
    /// handed out.
    ///
    /// # The notice is a prompt, not an instruction
    ///
    /// It says a handle went away, and the removal still checks that
    /// the entry under that number is the one that went away — because
    /// by the time this runs, the client may have opened a NEW scope
    /// with the same number, and removing by number alone would evict
    /// it. [`is_closed`](UnboundedSender::is_closed) tells the two
    /// apart, since a fresh entry has a live receiver behind it.
    ///
    /// # Why the check is never merely stale
    ///
    /// [`ScopeHandle`]'s [`Drop`] closes its receiver BEFORE sending
    /// the notice. So a notice that has arrived is a notice whose
    /// sender already reads as closed, and there is no window in which
    /// this looks at a scope that is on its way out and sees it as
    /// live.
    fn drain_closed(&mut self) {
        while let Ok(scope) = self.closed.try_recv() {
            if self
                .scopes
                .get(&scope)
                .is_some_and(UnboundedSender::is_closed)
            {
                self.scopes.remove(&scope);
            }
        }
    }
}

/// One scope at a time, until the connection ends.
///
/// [`None`] means the connection ended — a peer that closed and a peer
/// that vanished arrive the same way, and there is no other ending: a
/// client may open a scope at any moment for as long as it can write.
///
/// Frames that belong to scopes already open are delivered on the way
/// past, which is why polling this is what runs the connection rather
/// than merely what accepts from it.
///
/// Nothing in here waits except the socket read, which is what lets it
/// be a [`Stream`] at all. Delivering a frame into a scope is a
/// synchronous send onto an unbounded queue, so one scope that is not
/// being served slows nobody down — and grows instead, which is the
/// obligation that lands on
/// [`ScopeHandle`]: read your channel requests, or drop the scope.
///
/// # Discarding
///
/// A frame nobody is waiting for is dropped, silently, and the loop
/// carries on. There is no one to tell, and the connection is still
/// good for every other scope on it.
///
/// It happens six ways: a header too short to read, a type this layer
/// does not define (including `2` and `3`, which are a server's replies
/// and not a client's to send), an auth frame, a channel response of
/// either kind, a request for a scope that is already open, and a
/// channel request for a scope that is not.
///
/// The channel responses are the temporary one. Nothing here opens a
/// channel yet, so an answer to one is an answer to a question this end
/// never asked.
///
/// A duplicate scope is the peer's mistake rather than this end's, and
/// it is still silent. Handing out a second scope for one number would
/// mean two writers answering into one client stream and the first
/// finish orphaning the other; replacing the entry would starve whoever
/// holds the first. Both are worse than dropping, and a client that
/// does it sees a request that is never answered.
impl Stream for Session {
    type Item = ScopeHandle;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<ScopeHandle>> {
        // Every field is `Unpin` — the two socket halves, a map, and
        // two channel ends — so this never has to project.
        let this = self.get_mut();
        loop {
            let Some(received) = ready!(this.stream.poll_next_unpin(cx))
            else {
                return Poll::Ready(None);
            };
            // No frame, so nothing to route. A transport error yields
            // none, and the loop takes the next one.
            let Ok(bytes) = received else { continue };
            // Decoded for its header alone. The payload is borrowed and
            // then ignored; what gets forwarded is `bytes`, whole and
            // untouched — a consumer needs the header too, because
            // telling a request from a finish means reading the type.
            let Ok(frame) = ClientFrame::decode(&bytes) else { continue };
            match frame {
                // Nowhere to go, and nothing here opened anything one
                // of these could answer. See the type's documentation.
                ClientFrame::Auth { .. }
                | ClientFrame::ChannelResponse { .. }
                | ClientFrame::ChannelResponseFinish { .. } => {}
                ClientFrame::Request { scope, .. } => {
                    if let Some(handle) = this.open_scope(scope, bytes) {
                        return Poll::Ready(Some(handle));
                    }
                }
                // A send that fails is a scope whose handle went away
                // and whose entry has not been swept yet. Nothing to
                // do about either: the frame had nowhere to go, and
                // the entry goes the next time a scope opens.
                ClientFrame::ChannelRequest { scope, .. } => {
                    if let Some(sender) = this.scopes.get(&scope) {
                        let _ = sender.send(bytes);
                    }
                }
            }
        }
    }
}
