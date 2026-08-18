//! The read loop: frames in, and off to whoever is waiting.

use std::collections::HashMap;

use bytes::Bytes;
use futures_util::StreamExt as _;
use futures_util::stream::SplitStream;
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};

use crate::connection::Connection;
use crate::frame::client::ClientFrame;

/// One connection's inbound half.
///
/// Reads frames, looks up where each belongs, and forwards it. The
/// mirror of [`client::router::Router`](crate::client::router::Router),
/// and the same wherever the two halves are the same: whole frames go
/// on untouched, header included; one hop from here to the consumer;
/// bounded senders, so a consumer that stops reading stops the
/// connection rather than losing a frame; a lookup that misses drains
/// the registration queue and tries once more.
///
/// # What is not mirrored
///
/// **Scope-opening requests go somewhere before any scope exists.**
/// Nothing else can: the client chooses the scope, so this end has
/// nothing to register against until a request names one. They all go
/// to one stream, the only one on this connection that is not about a
/// particular scope.
///
/// **A scope ends from this end.** No client frame closes one — the
/// server's own response finish does, and this reads what the client
/// sends. So it has to be told, which is what
/// [`ScopeClosed`](Registration::ScopeClosed) is for.
///
/// **A closure only ever names a channel.** A scope this end closed is
/// a scope this end already knows about, so there is nothing to report
/// back about it. Hence `(scope, channel)` rather than the client
/// router's `(scope, Option<u32>)`.
///
/// **A scope registers one sender, not two.** Responses on channel `0`
/// are what this end SENDS, so there is nothing coming back on that
/// channel and nowhere for it to go.
///
/// # Auth is dropped on the floor
///
/// And that is a gap, not a decision. A credential belongs to the
/// connection, this reads the connection, and there is nowhere for it
/// to go — no channel out of here carries one, so the frame is
/// discarded like a type nobody defines. Whatever ends up deciding what
/// a credential means will need a path through here, and does not have
/// one.
#[derive(Debug)]
pub struct Router {
    /// The read half of the connection.
    ///
    /// Split, because the write half belongs to whoever is writing and
    /// a [`Sink`](futures_util::Sink) needs `&mut` — see
    /// [`connection`](crate::connection) for why that is a wire
    /// requirement rather than an implementation detail.
    stream: SplitStream<Connection>,
    /// Every scope open on this connection.
    scopes: HashMap<u32, Scope>,
    /// Where every scope-opening request goes.
    ///
    /// One stream for the connection, because a request is what creates
    /// a scope and there is nothing scope-shaped to sort it into yet.
    /// The scope is in the frame's header, and registering somewhere to
    /// put the rest of it is what whoever reads this does next.
    ///
    /// Bounded, and a full one stops the loop like any other. The cost
    /// of not stopping would be worse here than anywhere: a dropped
    /// request is a scope a client is waiting on that nobody is
    /// serving, and it would wait forever, because a stream ends at its
    /// finish frame and no finish would ever come.
    request_sender: Sender<Bytes>,
    /// Somewhere to register a new destination before its frames
    /// arrive.
    ///
    /// See [`Registration`] for the three kinds.
    ///
    /// Unbounded on purpose. Registering must not block, because
    /// whoever is registering is about to write the frame that causes
    /// the traffic — and a registration that waited behind a full queue
    /// would be an exchange whose other half arrives before anywhere
    /// exists to put it.
    registrations: UnboundedReceiver<Registration>,
    /// Where this says a channel is finished.
    ///
    /// `(scope, channel)`, in this end's numbering, sent when the
    /// client's answer on a channel this end opened runs out. It frees
    /// the number for reuse; the receiver closing says the same thing
    /// to whoever was reading the answer.
    ///
    /// Unbounded, because it is sent from inside the loop that would
    /// otherwise have to drain it.
    closed: UnboundedSender<(u32, u32)>,
}

impl Router {
    /// Take the read half, and the three ends that connect it to the
    /// writer.
    ///
    /// The read half of a split [`Connection`], the sending end of the
    /// request stream, the receiving end of the registration channel,
    /// and the sending end of the closure channel. None of them is made
    /// here: each has another end, and the other ends belong together
    /// elsewhere.
    ///
    /// No scopes. A connection starts with none open, and every entry
    /// arrives through `registrations`.
    pub fn new(
        stream: SplitStream<Connection>,
        request_sender: Sender<Bytes>,
        registrations: UnboundedReceiver<Registration>,
        closed: UnboundedSender<(u32, u32)>,
    ) -> Self {
        Router {
            stream,
            scopes: HashMap::new(),
            request_sender,
            registrations,
            closed,
        }
    }

    /// Read frames until the connection ends.
    ///
    /// Returns when the stream ends, and at no other point. A transport
    /// error yields no frame, so there is nothing to route and the loop
    /// takes the next one — see
    /// [`client::router::Router::run`](crate::client::router::Router::run)
    /// for why that is the rule rather than ending here.
    ///
    /// Every consumer learns the connection is over the same way: this
    /// is dropped, and every sender in it with it.
    ///
    /// # Discarding
    ///
    /// A frame nobody is waiting for is dropped, silently, and the loop
    /// carries on. It happens five ways: a header too short to read, a
    /// type this layer does not define (including `2` and `3`, which
    /// are a server's replies and not a client's to send), an auth
    /// frame, a scope with no entry, and a channel with no entry inside
    /// a scope that has one.
    ///
    /// A request is the one frame that cannot miss. It is not routed by
    /// scope, so there is no entry for it to fail to find — only a
    /// stream nobody is reading, which is a server with nothing serving
    /// it.
    pub async fn run(mut self) {
        while let Some(received) = self.stream.next().await {
            // No frame, so nothing to route. The stream is what ends
            // this loop, and it has not ended.
            let Ok(bytes) = received else { continue };
            // Decoded for its header alone. The payload is borrowed and
            // then ignored; what gets forwarded is `bytes`, whole and
            // untouched.
            let Ok(frame) = ClientFrame::decode(&bytes) else { continue };
            match frame {
                // Nowhere to go. See the type's documentation: this is
                // the gap, not a decision.
                ClientFrame::Auth { .. } => {}
                // Not routed by scope, because the scope is what it
                // opens. Nothing is inserted here either: the entry
                // arrives when whoever serves this says where the rest
                // of the scope should go.
                ClientFrame::Request { .. } => {
                    let _ = self.request_sender.send(bytes).await;
                }
                // The scope's stream, not a channel's. The channel
                // number in the header is the CLIENT's, and the map
                // holds this end's — see `request_sender`.
                //
                // Nothing is closed. There is no entry to close: a
                // request opens a channel that this end answers, and
                // both the answering and the closing happen where the
                // frames go, not here.
                ClientFrame::ChannelRequest { scope, .. } => {
                    if let Some(entry) = self.scope(scope) {
                        let _ = entry.request_sender.send(bytes).await;
                    }
                }
                ClientFrame::ChannelResponse { scope, channel, .. } => {
                    if let Some(sender) = self.channel(scope, channel) {
                        let _ = sender.send(bytes).await;
                    }
                }
                // Forwarded first, because the finish is what says the
                // answer ended rather than the connection.
                ClientFrame::ChannelResponseFinish { scope, channel } => {
                    if let Some(sender) = self.channel(scope, channel) {
                        let _ = sender.send(bytes).await;
                    }
                    self.close_channel(scope, channel);
                }
            }
        }
    }

    /// Find a scope, draining registrations first if it is not there.
    ///
    /// The retry is not an optimization, it is the fix for a race that
    /// is otherwise unavoidable: a request reaches its server through
    /// one queue and the registration comes back through another, and
    /// the client's next frame is racing both. Draining on a miss
    /// closes the window — anything registered before the frame was
    /// read is found on the second look.
    ///
    /// A miss after the drain is a real miss. Nothing waits for a
    /// registration that has not been made, because nothing says one is
    /// coming: a request that nobody chose to serve never gets one.
    fn scope(&mut self, scope: u32) -> Option<&mut Scope> {
        if !self.scopes.contains_key(&scope) {
            self.drain();
        }
        self.scopes.get_mut(&scope)
    }

    /// Find a channel inside a scope, on the same terms.
    ///
    /// Either half missing drains, because either half can be the one
    /// that has not arrived: a channel's registration is its own entry,
    /// and it queues behind the scope's.
    fn channel(&mut self, scope: u32, channel: u32) -> Option<&Sender<Bytes>> {
        let found = self
            .scopes
            .get(&scope)
            .is_some_and(|entry| entry.channels.contains_key(&channel));
        if !found {
            self.drain();
        }
        self.scopes.get(&scope)?.channels.get(&channel)
    }

    /// Drop a channel, and say so.
    ///
    /// Only on a real removal, so that whoever is counting what it
    /// opened against what it closed is never told twice.
    ///
    /// The scope stays. A scope outlives its channels — it is the
    /// request, and they are the exchanges inside it — and it ends when
    /// this end finishes it, which this end does not learn from here.
    fn close_channel(&mut self, scope: u32, channel: u32) {
        let Some(entry) = self.scopes.get_mut(&scope) else {
            return;
        };
        if entry.channels.remove(&channel).is_some() {
            let _ = self.closed.send((scope, channel));
        }
    }

    /// Take every registration that is waiting, and apply it.
    ///
    /// Registrations arrive unbounded and this empties the queue rather
    /// than taking one, because the cost is per call and not per item:
    /// it runs on a miss, and a miss is what it is trying to stop
    /// happening again.
    ///
    /// Two things get discarded, and neither is anybody's mistake here
    /// the way a duplicate is on the client's side.
    ///
    /// **A duplicate.** An entry that already exists stays, because
    /// replacing it would redirect a stream somebody is still reading.
    /// A client that reuses a live scope number is the usual cause, and
    /// the one that could have prevented it.
    ///
    /// **A channel with no scope.** Its scope was finished between the
    /// writer registering and this reading it, which is a race nobody
    /// could have avoided. There is nothing to do about it and nothing
    /// to report: a registration is not a request, and there is no
    /// reply to put an answer in.
    ///
    /// [`ScopeClosed`](Registration::ScopeClosed) is the one that
    /// removes rather than adds, and it shares this queue for a reason
    /// — see there.
    fn drain(&mut self) {
        while let Ok(registration) = self.registrations.try_recv() {
            match registration {
                Registration::Scope {
                    scope,
                    request_sender,
                } => {
                    self.scopes.entry(scope).or_insert_with(|| Scope {
                        request_sender,
                        channels: HashMap::new(),
                    });
                }
                Registration::Channel {
                    scope,
                    channel,
                    response_sender,
                } => {
                    let Some(entry) = self.scopes.get_mut(&scope) else {
                        continue;
                    };
                    entry.channels.entry(channel).or_insert(response_sender);
                }
                Registration::ScopeClosed { scope } => {
                    self.scopes.remove(&scope);
                }
            }
        }
    }
}

/// Where one scope's frames go.
///
/// Nested inside [`Router`]'s map rather than flattened into a
/// `(scope, channel)` key, because a scope ending ends everything under
/// it: one removal drops the channel requests and every channel at
/// once, where a flat map would have to be scanned for them.
#[derive(Debug)]
struct Scope {
    /// The channels the CLIENT opens inside this scope. All of them, on
    /// one stream.
    ///
    /// They cannot share the map with [`channels`](Self::channels). A
    /// channel number belongs to whoever opened it and both sides count
    /// from zero, so the client's channel `3` and this end's channel
    /// `3` are two different channels, and one map keyed by `3` holds
    /// one of them.
    ///
    /// Not that the number is lost: it is in the header of the frame
    /// that goes down this, whole. What this end does with it is answer
    /// on it, which is a write, and writes are not a router's business.
    ///
    /// Nothing is ever removed for one of these. A request is the whole
    /// of what arrives — one frame, opening a channel whose remaining
    /// traffic travels the other way — so there is no entry to keep and
    /// none to take away.
    request_sender: Sender<Bytes>,
    /// The channels this SERVER opened, by number, and what the client
    /// sends back on each.
    ///
    /// One space, because one opener: everything in here was numbered
    /// by this end. What the client numbers goes to
    /// [`request_sender`](Self::request_sender) instead, which is what
    /// keeps that true.
    channels: HashMap<u32, Sender<Bytes>>,
}

/// Somewhere to put frames, arranged before any of them arrive.
///
/// Sent to a [`Router`], rather than installed by one, because the
/// party that knows what a scope needs is the party that read the
/// request and decided to serve it. It races the frames it is for,
/// which a [`Router`] handles by draining this queue whenever a lookup
/// misses.
///
/// # Why a scope brings one sender, where a client's brings two
///
/// Because responses on channel `0` are what this end sends. A client
/// registers somewhere for them to arrive; here there is nothing
/// arriving on that channel at all, and the only thing a scope receives
/// is the channels the client opens inside it.
#[derive(Debug)]
pub enum Registration {
    /// Take a scope this end has decided to serve.
    ///
    /// Nothing routes into a scope until this arrives — not because the
    /// scope is not open, but because there is nowhere to put what
    /// arrives in it. The request itself has already gone out on its
    /// own stream.
    Scope {
        /// The scope, as the client numbered it.
        scope: u32,
        /// Where the client's channel requests go, whatever it numbers
        /// them.
        request_sender: Sender<Bytes>,
    },
    /// Open a channel inside a scope that is already registered.
    ///
    /// Discarded if it is not. A channel entry lives inside a scope's
    /// entry, and a [`Router`] will not invent the scope to put it in.
    Channel {
        /// The scope it is inside.
        scope: u32,
        /// The channel, chosen by this end.
        channel: u32,
        /// Where the client's answers on it go.
        response_sender: Sender<Bytes>,
    },
    /// Forget a scope. This end has finished it.
    ///
    /// The one variant that removes, and it is here rather than on a
    /// queue of its own because the order matters: a scope closed and a
    /// scope opened are two edits to the same key, and a client is free
    /// to reuse a number the moment it sees the finish. Split across
    /// two queues, a close could overtake the registration that follows
    /// it and quietly delete a scope that had just been reopened.
    ///
    /// Everything under it goes with it, and nothing is reported back —
    /// the party that would be told is the party that just said this.
    ScopeClosed {
        /// The scope that is over.
        scope: u32,
    },
}
