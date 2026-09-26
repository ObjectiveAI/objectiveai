//! The read loop: frames in, and off to whoever is waiting.

use std::collections::HashMap;
use std::fmt;

use bytes::Bytes;
use futures_util::StreamExt as _;
use futures_util::stream::SplitStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::registration::Registration;
use crate::wire::connection::Connection;
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
/// thousands — and it would not buy backpressure isolation, because
/// the queue feeding that task fills just the same.
///
/// # Nothing here waits
///
/// Every sender is unbounded, so forwarding a frame is a push and this
/// loop never blocks on a consumer. One scope that has stopped reading
/// cannot stall another, and cannot stall the socket.
///
/// Dropping the frame was never on the table — a stream has no way to
/// say it lost one, and a caller reassembling an image layer would get
/// a shorter layer with no indication of it. What replaces the stall is
/// memory: an unread queue grows, at the rate the far end is sending.
///
/// So the requirement on consumers is sharper than it was, not softer:
/// read your receiver, or drop it. A dropped one makes these sends fail,
/// which is ignored, and frees what it was holding.
///
/// # When entries leave
///
/// A finish takes one out, which is the only removal there is, and a
/// closure goes out to whoever is keeping a matching record. A consumer
/// that walked away without a finish lingers — a send to a dropped
/// receiver fails and that failure is ignored, and no send happens at
/// all if no further frame arrives — so a scope nobody is listening to
/// occupies its entry until the connection ends.
#[derive(Debug)]
pub struct Router {
    /// The read half of the connection.
    ///
    /// Split, because the write half belongs to whoever is writing and
    /// a [`Sink`](futures_util::Sink) needs `&mut` — see
    /// [`connection`](crate::wire::connection) for why that is a wire
    /// requirement rather than an implementation detail.
    stream: SplitStream<Connection>,
    /// Every scope open on this connection.
    ///
    /// `(response_sender, request_sender, channels)`.
    ///
    /// **`0`** is the answer to the request that opened the scope.
    /// Channel `0`, which the frame layer already treats as the scope's
    /// own, so it does not need a key in the map and does not get one.
    ///
    /// **`1`** is every channel the SERVER opens inside this scope, on
    /// one stream. They cannot share the map with `2`: a channel number
    /// belongs to whoever opened it and both sides count from zero, so
    /// the server's channel `3` and this client's channel `3` are two
    /// different channels, and one map keyed by `3` holds one of them.
    /// The number is not lost — it is in the header of the frame that
    /// goes down this, whole — and nothing is ever removed for one,
    /// because a request is one frame and its answer travels the other
    /// way.
    ///
    /// **`2`** is the channels this CLIENT opened, by number, and what
    /// the server sends back on each. One space, because one opener:
    /// everything in it was numbered by this end, which is what `1`
    /// being separate keeps true.
    ///
    /// Nested rather than flattened into a `(scope, channel)` key,
    /// because a scope ending ends everything under it: one removal
    /// drops both senders and every channel at once, where a flat map
    /// would have to be scanned for them.
    scopes: HashMap<
        u32,
        (UnboundedSender<Bytes>, UnboundedSender<Bytes>, HashMap<u32, UnboundedSender<Bytes>>),
    >,
    /// Somewhere to register a new destination before its frames
    /// arrive.
    ///
    /// See [`Registration`] for the two kinds.
    ///
    /// Unbounded on purpose. Registering must not block, because
    /// whoever is registering is about to write the request that
    /// causes the frames — and a registration that waited behind a
    /// full queue would be a request whose answers arrive before
    /// anywhere exists to put them.
    ///
    /// Which is now true of every queue here rather than only this one.
    /// It stays worth saying because this one would have to be
    /// unbounded even if the others were not: it is drained from inside
    /// the loop that fills the rest.
    registration_receiver: UnboundedReceiver<Registration>,
    /// Somewhere to say an entry is gone.
    ///
    /// `(scope, channel)`, naming what a [`Registration`] named —
    /// `None` for the scope itself, which takes its channels with it
    /// and does not announce them one by one.
    ///
    /// It is a fact the holder cannot work out alone. A finish arrives
    /// here and nowhere else, and what a scope or channel number MEANS
    /// is a client's own bookkeeping: which numbers are in use, what
    /// each one was asked for. Closing the receiver tells it a stream
    /// ended; this tells it the number is free.
    ///
    /// Unbounded, and for a plainer reason than
    /// [`registration_receiver`](Self::registration_receiver) — this is sent
    /// inside the read loop, so a bounded queue that filled would stop
    /// the loop that drains it. That reasoning is now shared with every
    /// other queue in here, which is unbounded for a broader one. Nothing is retried and a failed send is
    /// ignored: the only way to fail is a holder that is gone, and a
    /// holder that is gone has no record to correct.
    closed_sender: UnboundedSender<(u32, Option<u32>)>,
}

impl Router {
    /// Take the three parts a router is made of.
    ///
    /// The read half of a split [`Connection`], the receiving end of
    /// the registration channel, and the sending end of the closure
    /// channel — one socket and both directions of the traffic about
    /// it. Whoever holds the other ends of those two is whoever asks
    /// for frames and hears when they stop.
    ///
    /// None of them is made here, and that is the point: the split
    /// produces a writer at the same moment, and each channel has an
    /// end that has to go somewhere. All of it belongs to the caller,
    /// which pairs this with them and keeps what it is not handing
    /// over.
    ///
    /// No scopes. A connection starts with none open, and every entry
    /// arrives through `registration_receiver`.
    pub fn new(
        stream: SplitStream<Connection>,
        registration_receiver: UnboundedReceiver<Registration>,
        closed_sender: UnboundedSender<(u32, Option<u32>)>,
    ) -> Self {
        Router {
            stream,
            scopes: HashMap::new(),
            registration_receiver,
            closed_sender,
        }
    }

    /// Read frames until the connection ends.
    ///
    /// Returns when the stream ends — or, the one early exit, when a
    /// credential arrives. A connection has ONE, presented by the side
    /// that dialled, before this router existed:
    /// [`authorize`](super::authorize::authorize) already consumed the
    /// only lawful one, so a provider sending one now is disagreeing
    /// with this end about what the handshake is, and nothing after
    /// that disagreement is worth routing. Either way this is dropped,
    /// and with it every sender in it — a peer that closed, a peer
    /// that vanished, and a peer that broke the handshake all arrive
    /// at every consumer the same way: a channel that closed.
    ///
    /// A transport error is not the end. It yields no frame, so there
    /// is nothing to route and the loop takes the next one — which is
    /// the end, in practice, because both transports fuse: an error
    /// sets `ended` and the poll after it returns `None`
    /// ([`tokio_tungstenite::WebSocketStream`], which
    /// [`axum`](axum::extract::ws::WebSocket) delegates to).
    ///
    /// Which makes ending here and reading on identical today, and the
    /// choice is about what the identity rests on. Reading on is right
    /// because the stream says when it is over; ending on an error is
    /// right only while every transport behind [`Connection`] fuses,
    /// which is a fact about two dependencies rather than about this
    /// protocol.
    ///
    /// A closed channel is how a connection ending differs from a
    /// stream ending. A stream ends at its finish frame, which the
    /// consumer sees before the close; a consumer whose channel closes
    /// without one lost the connection mid-stream.
    ///
    /// # Discarding
    ///
    /// A frame nobody is waiting for is dropped, silently, and the loop
    /// carries on. There is no one to tell — the party that would care
    /// is the one that is not there — and the connection is still good
    /// for every other scope on it.
    ///
    /// It happens four ways: a header too short to read, a type this
    /// layer does not define (including `1`, which is a client's to
    /// send and not a server's), a scope with no entry, and a channel
    /// with no entry inside a scope that has one. The last two are the
    /// ordinary ones — a scope ends and its late frames arrive after,
    /// or a consumer never registered at all.
    pub async fn run(mut self) -> Result<(), InvalidAuthorize> {
        while let Some(received) = self.stream.next().await {
            // No frame, so nothing to route. The stream is what ends
            // this loop, and it has not ended.
            let Ok(bytes) = received else { continue };
            // Decoded for its header alone. The payload is borrowed and
            // then ignored; what gets forwarded is `bytes`, whole and
            // untouched.
            let Ok(frame) = ServerFrame::decode(&bytes) else { continue };
            match frame {
                // The connection's one credential was spent before this
                // router existed — see `run`. Ending here drops every
                // sender, which is how consumers already learn a
                // connection went.
                ServerFrame::Auth { .. } => {
                    return Err(InvalidAuthorize);
                }
                ServerFrame::Response { scope, .. } => {
                    if let Some((response_sender, ..)) = self.scope(scope) {
                        let _ = response_sender.send(bytes);
                    }
                }
                // The scope is over, so everything under it goes: both
                // its senders and every channel that was open inside it.
                // Forwarded first, because the finish is what says the
                // stream ended rather than the connection.
                ServerFrame::ResponseFinish { scope } => {
                    if let Some((response_sender, ..)) = self.scope(scope) {
                        let _ = response_sender.send(bytes);
                    }
                    self.close_scope(scope);
                }
                // The scope's sender, not a channel's. The channel
                // number in the header is the SERVER's, and the map
                // holds the client's — see `request_sender`.
                //
                // Nothing is closed, because nothing was opened here. A
                // request is one frame, and what answers it travels the
                // other way, where this never looks.
                ServerFrame::ChannelRequest { scope, .. } => {
                    if let Some((_, request_sender, _)) = self.scope(scope) {
                        let _ = request_sender.send(bytes);
                    }
                }
                ServerFrame::ChannelResponse { scope, channel, .. } => {
                    if let Some(sender) = self.channel(scope, channel) {
                        let _ = sender.send(bytes);
                    }
                }
                // As `ResponseFinish`, one level down: forward, then
                // drop the channel and leave the scope open.
                ServerFrame::ChannelResponseFinish { scope, channel } => {
                    if let Some(sender) = self.channel(scope, channel) {
                        let _ = sender.send(bytes);
                    }
                    self.close_channel(scope, channel);
                }
            }
        }
        Ok(())
    }

    /// Find a scope, draining registrations first if it is not there.
    ///
    /// The retry is not an optimization, it is the fix for a race that
    /// is otherwise unavoidable: a registration and the request that
    /// causes its frames travel by different routes, and the answer can
    /// beat the registration here. Draining on a miss closes that
    /// window — anything registered before the frame was read is found
    /// on the second look.
    ///
    /// A miss after the drain is a real miss. Nothing waits for a
    /// registration that has not been made, because nothing says one is
    /// coming.
    fn scope(
        &mut self,
        scope: u32,
    ) -> Option<
        &mut (UnboundedSender<Bytes>, UnboundedSender<Bytes>, HashMap<u32, UnboundedSender<Bytes>>),
    > {
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
    fn channel(&mut self, scope: u32, channel: u32) -> Option<&UnboundedSender<Bytes>> {
        let found = self
            .scopes
            .get(&scope)
            .is_some_and(|entry| entry.2.contains_key(&channel));
        if !found {
            self.drain();
        }
        self.scopes.get(&scope)?.2.get(&channel)
    }

    /// Drop a scope, and every channel that was open inside it.
    ///
    /// One removal does both, which is the whole reason the channels
    /// are nested rather than keyed by `(scope, channel)` — see
    /// `scopes`. The map entry going away is what the consumers see: a
    /// dropped [`Sender`] closes its receiver, so each of them learns
    /// its stream is over without being told individually.
    ///
    /// A scope that is already gone is not an error. A finish for a
    /// scope nobody registered is the ordinary case, not a special one.
    /// It is also not a closure, and nothing is announced for it —
    /// [`closed`](Self::closed) reports entries that this removed, so
    /// that a holder counting what it opened against what it closed
    /// gets the same number twice.
    fn close_scope(&mut self, scope: u32) {
        if self.scopes.remove(&scope).is_some() {
            let _ = self.closed_sender.send((scope, None));
        }
    }

    /// Drop a channel, and leave the scope it was in alone.
    ///
    /// A scope outlives its channels — it is the request, and they are
    /// the exchanges inside it — so a channel ending takes nothing else
    /// with it. A scope that is already gone took this with it.
    fn close_channel(&mut self, scope: u32, channel: u32) {
        let Some(entry) = self.scopes.get_mut(&scope) else { return };
        if entry.2.remove(&channel).is_some() {
            let _ = self.closed_sender.send((scope, Some(channel)));
        }
    }

    /// Take every registration that is waiting, and add what it can.
    ///
    /// Registrations arrive unbounded and this empties the queue rather
    /// than taking one, because the cost is per call and not per item:
    /// it runs on a miss, and a miss is what it is trying to stop
    /// happening again.
    ///
    /// Two things get discarded, and both are the same mistake made by
    /// whoever registered.
    ///
    /// **A duplicate.** An entry that already exists stays. Replacing
    /// it would silently redirect a stream someone else is still
    /// reading, and the two claimants cannot both be right — the scope
    /// or channel number is a client's to choose, so a collision is a
    /// client that reused a live one.
    ///
    /// **A channel with no scope.** A channel entry needs the scope's
    /// entry to live in, and this does not invent one — a scope with no
    /// registration of its own is a scope whose responses have nowhere
    /// to go, so half an entry would be a channel that works inside a
    /// scope that never did.
    ///
    /// Neither is reported, because a registration is not a request and
    /// there is no reply to put an answer in. What a caller sees is a
    /// receiver that stays empty.
    fn drain(&mut self) {
        while let Ok(registration) = self.registration_receiver.try_recv() {
            match registration {
                Registration::Scope {
                    scope,
                    response_sender,
                    request_sender,
                } => {
                    self.scopes.entry(scope).or_insert_with(|| {
                        (response_sender, request_sender, HashMap::new())
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
                    entry.2.entry(channel).or_insert(response_sender);
                }
            }
        }
    }
}

/// A credential arrived after the handshake.
///
/// What [`Router::run`] returns instead of ending cleanly, and the
/// caller-half mirror of the server's
/// [`HandleError::InvalidAuthorize`](crate::provider::server::handle::HandleError::InvalidAuthorize):
/// a connection has one credential, presented by the side that dialled
/// before anything else, and
/// [`authorize`](super::authorize::authorize) already consumed it. A
/// provider presenting another is disagreeing about what the handshake
/// is, and routing stopped where the disagreement started.
///
/// It reaches the party that spawned the router and nobody else — in
/// particular, not the provider, which is owed no reply on the subject
/// of credentials, and not the consumers, who see exactly what a
/// vanished connection shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvalidAuthorize;

impl fmt::Display for InvalidAuthorize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a credential arrived after the handshake")
    }
}

impl std::error::Error for InvalidAuthorize {}
