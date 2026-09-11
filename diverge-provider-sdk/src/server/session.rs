//! One connection, as the scopes a client opens on it.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt as _, Stream, StreamExt as _};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::notice::Notice;
use super::received::Received;
use super::scope_handle::ScopeHandle;
use crate::connection::Connection;
use crate::encode::{Encode, Writer};
use crate::frame::auth::Auth;
use crate::frame::client::ClientFrame;
use crate::frame::server::ServerFrame;

/// A connection's provider side: what a client starts on it.
///
/// A [`Stream`] of [`Received`] — a credential, or a request beside
/// the [`ScopeHandle`] that answers it — and that is the whole of a
/// provider's outer loop: take an item, judge or serve it, take the
/// next. See [`Received`] for why there are exactly two kinds and why
/// the ordering rules between them are not enforced here.
///
/// # Why this is not a router
///
/// The caller half has one, and it earns its keep there: a client
/// arranges where a scope's frames will go BEFORE opening it, so
/// something has to hold those arrangements and match arriving frames
/// against them. That is routing, and it is a job.
///
/// Nothing here can arrange a SCOPE. A server does not open one, it is
/// told about one — so what would have been a router is a loop that
/// reads frames and hands out the ones that open something.
///
/// Channels are the other way round, and that is why there is a
/// registration queue: a channel this end opens is this end's to
/// number, and the answer to it has to have somewhere to land before
/// it arrives.
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
/// # Auth flows through, in both directions
///
/// A peer's credential arrives as [`Received::Auth`] and is judged by
/// whoever holds this stream; the credential a provider owes on an
/// [`Outgoing`](crate::connection::Connection::Outgoing) connection
/// goes out through the session too, sent by
/// [`handle`](super::handle::handle) before it reads. This type carries
/// them and takes no position on either — the handshake's rules live
/// with the handshake, in [`handle`](super::handle::handle).
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
    /// Every scope a client has open, and everything routed inside it.
    ///
    /// `(channel_request_sender, channels, finished_channel_sender)`.
    ///
    /// **`0`** is where the channels the CLIENT opens go, whole and in
    /// arrival order. One stream for all of them, because the number in
    /// each header is the client's and this end was never told about
    /// any of them.
    ///
    /// **`1`** is the channels THIS end opened, by number, and where
    /// the client's answers on each go. A separate space, because a
    /// channel number belongs to whoever opened it and both sides count
    /// from zero — the client's channel `3` and this end's channel `3`
    /// are two different channels, and one map keyed by `3` would hold
    /// one of them.
    ///
    /// **`2`** is how a number gets given back. When an answer
    /// finishes, the channel it finished is free to be minted again,
    /// and the only party that learns it is this one — so it says so,
    /// to the handle that does the minting. The mirror of
    /// [`client::router::Router`](crate::client::router::Router)'s
    /// `closed`, one level down and per scope rather than per
    /// connection.
    ///
    /// Nested rather than flattened to a `(scope, channel)` key,
    /// because a scope ending ends everything under it: one removal
    /// drops the inbox and every channel at once, where a flat map
    /// would have to be scanned. It is the same shape, for the same
    /// reason, as
    /// [`client::router::Router`](crate::client::router::Router)'s.
    ///
    /// An address book rather than bookkeeping. It holds no counter and
    /// frees no numbers: the scope numbers are the client's, and a
    /// scope's channel numbers belong to the one handle that mints
    /// them.
    ///
    /// # Unbounded, and what that trades
    ///
    /// Both queues, so that delivering a frame is a synchronous send
    /// that cannot wait. [`poll_next`](Stream::poll_next) cannot wait
    /// either, and a bounded queue would mean carrying a
    /// half-delivered frame across polls — a stored reservation, an
    /// allocation, and a stall that stops the whole connection whenever
    /// any one scope falls behind.
    ///
    /// What it costs is a bound, and the two queues owe differently for
    /// it. A channel request is ONE frame, so that queue holds channels
    /// opened and not yet taken. A channel RESPONSE is a stream — an
    /// image layer, a database connection, a command's items — so an
    /// answer nobody reads accumulates without limit, and it is the
    /// larger exposure by far.
    ///
    /// Nothing here can bound either, and nothing here tries. The
    /// remedy is to read a queue or drop the thing holding it, which
    /// frees it.
    scopes: HashMap<
        u32,
        (
            UnboundedSender<Bytes>,
            HashMap<u32, UnboundedSender<Bytes>>,
            UnboundedSender<u32>,
        ),
    >,
    /// Everything the scopes have to say, in the order they said it.
    ///
    /// See [`Notice`] for the two kinds and
    /// [`drain`](Self::drain) for how they are applied.
    ///
    /// Unbounded, because half of what rides it is sent from a
    /// destructor, where there is nothing to await on and nowhere to
    /// report a failure. The other half must not block either: a
    /// registration that waited behind a full queue would be a channel
    /// request whose answer arrives before anywhere exists to put it.
    notice_receiver: UnboundedReceiver<Notice>,
    /// The other end of it, kept to clone into every scope.
    notice_sender: UnboundedSender<Notice>,
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
        let (notice_sender, notice_receiver) = mpsc::unbounded_channel();
        Session {
            stream,
            sink: Arc::new(Mutex::new(sink)),
            scopes: HashMap::new(),
            notice_receiver,
            notice_sender,
        }
    }

    /// Make a scope's inbox, and hand it out.
    ///
    /// [`None`] for a scope that is already open, which is the whole of
    /// what can go wrong here. A scope this end did not create, holding
    /// a number this end does not mint, needs nothing decided about it
    /// beyond somewhere to put what arrives inside.
    ///
    /// # Occupied is not the same as open
    ///
    /// An entry whose inbox reports
    /// [`is_closed`](UnboundedSender::is_closed) is a scope whose
    /// handle is gone, and it is overwritten rather than refused. A
    /// client is free to reuse a scope number once that scope has
    /// ended, and refusing on the strength of an entry nobody holds
    /// would refuse a legitimate request over bookkeeping that has not
    /// caught up.
    ///
    /// Overwriting takes the old scope's channels with it, which is
    /// correct: they belonged to a scope that no longer exists, and
    /// nothing will ever answer on them again.
    fn open_scope(&mut self, scope: u32) -> Option<ScopeHandle> {
        let occupied = self
            .scopes
            .get(&scope)
            .is_some_and(|(channel_request_sender, ..)| !channel_request_sender.is_closed());
        if occupied {
            return None;
        }
        let (channel_request_sender, channel_request_receiver) = mpsc::unbounded_channel();
        let (finished_channel_sender, finished_channel_receiver) =
            mpsc::unbounded_channel();
        self.scopes
            .insert(
            scope,
            (channel_request_sender, HashMap::new(), finished_channel_sender),
        );
        Some(ScopeHandle::new(
            scope,
            channel_request_receiver,
            finished_channel_receiver,
            self.notice_sender.clone(),
            self.sink.clone(),
        ))
    }

    /// Present this end's credential, as the connection's first frame.
    ///
    /// The one thing a session ever writes: everything else a provider
    /// says goes through the [`ScopeHandle`]s it hands out, and a
    /// credential belongs to the connection rather than to any scope.
    /// [`handle`](super::handle::handle) calls it exactly once, on an
    /// [`Outgoing`](super::authorization::Authorization::Outgoing)
    /// connection, before it reads anything — "nothing may precede it"
    /// is the frame's own rule.
    ///
    /// There is no answer to wait for and no failure to report: an
    /// accepted credential is followed by the connection simply
    /// working, a rejected one by a close, and a send that did not
    /// land is a connection that is already over — which the very next
    /// read will say.
    pub(super) async fn send_auth(&self, auth: Auth<'_>) {
        let mut payload = Vec::new();
        auth.encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|error| match error {});
        let mut buffer = Vec::new();
        ServerFrame::Auth { payload: &payload }
            .encode(&mut Writer::new(&mut buffer))
            .unwrap_or_else(|error| match error {});
        let _ = self.sink.lock().await.send(Bytes::from(buffer)).await;
    }

    /// Find a channel this end opened.
    ///
    /// No retry, unlike
    /// [`client::router::Router`](crate::client::router::Router)'s,
    /// which drains on a miss because a registration can otherwise
    /// arrive after the answer it is for. That race is closed here by
    /// ordering rather than by looking twice — see
    /// [`drain`](Self::drain).
    ///
    /// So a miss is a real miss: a channel this end never opened, one
    /// whose answer already finished, or one whose scope did.
    fn channel(
        &self,
        scope: u32,
        channel: u32,
    ) -> Option<&UnboundedSender<Bytes>> {
        self.scopes.get(&scope)?.1.get(&channel)
    }

    /// Drop a channel, and give its number back.
    ///
    /// A scope outlives its channels — it is the request, and they are
    /// the exchanges inside it — so a channel ending takes nothing else
    /// with it. A scope that is already gone took this with it.
    ///
    /// Unguarded, unlike the removals in [`drain`](Self::drain),
    /// because this is not acting on a message that may have gone
    /// stale: the finish frame that prompts it named the channel that
    /// is in the map right now.
    ///
    /// # Why the number goes back from HERE
    ///
    /// Because a finish is the only thing that makes a channel number
    /// safe to mint again. It is the client saying it will send nothing
    /// more under that number — so reusing it cannot collide with
    /// anything still in flight.
    ///
    /// A [`Channel`](super::scope_handle::Channel) being dropped is not
    /// that. It says a consumer walked away, which the client was never
    /// told, so the client may still be sending; a number freed on that
    /// signal could be minted again and would route the old channel's
    /// late frames into the new one. So an abandoned channel keeps its
    /// number until the connection ends, exactly as it does on the
    /// client.
    ///
    /// Only on a real removal, so that whoever is counting what it
    /// opened against what it closed is never told twice.
    fn close_channel(&mut self, scope: u32, channel: u32) {
        let Some((_, channels, finished_channel_sender)) =
            self.scopes.get_mut(&scope)
        else {
            return;
        };
        if channels.remove(&channel).is_some() {
            let _ = finished_channel_sender.send(channel);
        }
    }

    /// Apply everything the scopes have said, in the order they said
    /// it.
    ///
    /// The queue is emptied rather than sampled, because what it holds
    /// is exactly the work that is outstanding.
    ///
    /// # Once per frame, and why that is enough
    ///
    /// [`send_channel_request`](super::scope_handle::ScopeHandle::send_channel_request)
    /// enqueues its registration BEFORE the frame reaches the socket.
    /// So the chain is: registration queued, frame written, client
    /// reads it, client answers, this reads the answer — and by the
    /// time an answer is in hand, the registration it needs has been in
    /// the queue for at least a round trip. Draining after the read and
    /// before the lookup therefore always finds it.
    ///
    /// Which is why there is no drain-on-miss here, where
    /// [`client::router::Router`](crate::client::router::Router) has
    /// one. That router looks twice because it drains lazily; this
    /// drains on a schedule that already dominates the race.
    ///
    /// # Why not more rarely
    ///
    /// It was once per scope opened, back when a scope ending was the
    /// only thing that could accumulate. It is not any more: a
    /// [`Channel`](super::scope_handle::Channel) dropped sends a notice
    /// too, at a rate that has nothing to do with scopes opening.
    ///
    /// And the lazy trigger would almost never fire. On a healthy
    /// connection every lookup HITS, so a drain that ran only on a miss
    /// would not run at all — one long-lived scope opening ten thousand
    /// channels would hold ten thousand notices and ten thousand dead
    /// entries, and on a single-scope connection nothing would ever
    /// clear them.
    ///
    /// What it costs instead is one `try_recv` on an empty queue per
    /// frame — a couple of atomic loads, against a path that already
    /// decodes a header and hashes a scope number.
    ///
    /// # One queue is what makes the order right
    ///
    /// A handle registers a channel and later says the scope is over.
    /// On two queues those could be drained the wrong way round, and a
    /// registration would land after the eviction that should have
    /// covered it — an entry outliving the scope it belongs to. One
    /// FIFO makes that unrepresentable rather than merely documented.
    ///
    /// # A registration with no scope is dropped
    ///
    /// A channel entry lives inside a scope's entry, and this does not
    /// invent one: a scope that is not in the map is a scope that
    /// ended, and half an entry would be a channel working inside a
    /// scope that does not.
    ///
    /// # A closure is a prompt, not an instruction
    ///
    /// It says something went away, and the removal still checks that
    /// what is under that number is what went away.
    ///
    /// The hazard is not within a scope. Channel numbers there are
    /// minted by one counter that only ever goes up, so a stale
    /// closure cannot name a live channel of the same scope. It is
    /// across scope GENERATIONS: a client reuses scope `7`, the new
    /// handle mints channel `1` as every handle does, and a
    /// `Closed(7, Some(1))` left over from the old generation would
    /// evict it. The same reuse is what makes the scope case need a
    /// guard too.
    ///
    /// [`is_closed`](UnboundedSender::is_closed) tells the generations
    /// apart, since anything fresh has a live receiver behind it.
    ///
    /// The check is never merely stale, because both destructors close
    /// their receiver BEFORE sending. A notice that has arrived is a
    /// notice whose sender already reads as closed — which is what
    /// makes the guard a test of identity rather than of timing.
    fn drain(&mut self) {
        while let Ok(notice) = self.notice_receiver.try_recv() {
            match notice {
                Notice::Register {
                    scope,
                    channel,
                    response_sender,
                } => {
                    if let Some((_, channels, _)) = self.scopes.get_mut(&scope) {
                        channels.insert(channel, response_sender);
                    }
                }
                Notice::Closed(scope, None) => {
                    let gone = self
                        .scopes
                        .get(&scope)
                        .is_some_and(|(channel_request_sender, ..)| channel_request_sender.is_closed());
                    if gone {
                        self.scopes.remove(&scope);
                    }
                }
                Notice::Closed(scope, Some(channel)) => {
                    if let Some((_, channels, _)) = self.scopes.get_mut(&scope)
                        && channels
                            .get(&channel)
                            .is_some_and(UnboundedSender::is_closed)
                    {
                        channels.remove(&channel);
                    }
                }
            }
        }
    }
}

/// One [`Received`] at a time, until the connection ends.
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
/// be a [`Stream`] at all. Delivering a frame is a synchronous send
/// onto an unbounded queue, so one scope that is not being served slows
/// nobody down — and grows instead, which is the obligation that lands
/// on whoever holds the other end: read it, or drop it.
///
/// # Discarding
///
/// A frame nobody is waiting for is dropped, silently, and the loop
/// carries on. There is no one to tell, and the connection is still
/// good for every other scope on it.
///
/// It happens four ways: a header too short to read, a type this layer
/// does not define (including `2` and `3`, which are a server's replies
/// and not a client's to send), a request for a scope that is already
/// open, and anything inside a scope or a channel with no entry.
///
/// A duplicate scope is the peer's mistake rather than this end's, and
/// it is still silent. Handing out a second scope for one number would
/// mean two writers answering into one client stream and the first
/// finish orphaning the other; replacing the entry would starve whoever
/// holds the first. Both are worse than dropping, and a client that
/// does it sees a request that is never answered.
impl Stream for Session {
    type Item = Received;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Received>> {
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
            // Once per frame, before anything is looked up. See
            // `drain` for why here and why that is enough.
            this.drain();
            match frame {
                // Yielded like a request, judged by whoever reads this
                // stream. Whether it was allowed to arrive NOW is the
                // handshake's rule, not a router's.
                ClientFrame::Auth { payload } => {
                    let payload = bytes.slice_ref(payload);
                    return Poll::Ready(Some(Received::Auth(payload)));
                }
                ClientFrame::Request { scope, payload } => {
                    if let Some(handle) = this.open_scope(scope) {
                        // A refcounted view of exactly the payload —
                        // the tag and the request's bytes, the header
                        // already behind it.
                        let payload = bytes.slice_ref(payload);
                        return Poll::Ready(Some(Received::Request(
                            payload, handle,
                        )));
                    }
                }
                // A send that fails is a scope whose handle went away
                // and whose entry has not been swept yet. Nothing to
                // do about either: the frame had nowhere to go, and
                // the entry goes the next time a scope opens.
                ClientFrame::ChannelRequest { scope, .. } => {
                    if let Some((channel_request_sender, ..)) = this.scopes.get(&scope) {
                        let _ = channel_request_sender.send(bytes);
                    }
                }
                ClientFrame::ChannelResponse { scope, channel, .. } => {
                    if let Some(sender) = this.channel(scope, channel) {
                        let _ = sender.send(bytes);
                    }
                }
                // Forwarded first, because the finish is what says the
                // answer ended rather than the connection.
                ClientFrame::ChannelResponseFinish { scope, channel } => {
                    if let Some(sender) = this.channel(scope, channel) {
                        let _ = sender.send(bytes);
                    }
                    this.close_channel(scope, channel);
                }
            }
        }
    }
}
