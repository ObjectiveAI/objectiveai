//! The read loop: frames in, and off to whoever is waiting.

use std::collections::HashMap;

use bytes::Bytes;
use futures_util::StreamExt as _;
use futures_util::stream::SplitStream;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};

use crate::connection::Connection;
use crate::frame::server::ServerFrame;

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
/// [`ClientFrame`](crate::frame::client::ClientFrame) borrows from the
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
/// # A full queue waits
///
/// The senders are bounded and [`run`](Self::run) awaits them, so a
/// consumer that stops reading stops the whole connection — every
/// other scope on it included.
///
/// That is the deliberate half of the trade. The alternative is
/// dropping the frame, and a stream has no way to say it lost one: a
/// caller reassembling an image layer would get a shorter layer and no
/// indication of it. A stall is visible and recoverable; a hole is
/// neither.
///
/// What follows from it is a requirement on consumers rather than on
/// this: read your receiver, or close it.
///
/// # When entries leave
///
/// A finish takes one out, which is the only removal there is. A
/// consumer that walked away without one lingers — a send to a dropped
/// receiver fails and that failure is ignored, and no send happens at
/// all if no further frame arrives — so a scope nobody is listening to
/// occupies its entry until the connection ends.
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
    /// Somewhere to register a new destination before its frames
    /// arrive.
    ///
    /// `(scope, channel, sender)`, where the channel is `None` for the
    /// scope's own responses.
    ///
    /// Unbounded on purpose. Registering must not block, because
    /// whoever is registering is about to write the request that
    /// causes the frames — and a registration that waited behind a
    /// full queue would be a request whose answers arrive before
    /// anywhere exists to put them.
    registrations: UnboundedReceiver<(u32, Option<u32>, Sender<Bytes>)>,
}

impl Router {
    /// Take the two halves a router is made of.
    ///
    /// The read half of a split [`Connection`], and the receiving end
    /// of the registration channel — whoever holds the matching sender
    /// is whoever gets to ask for frames.
    ///
    /// Neither is made here, and that is the point: the split produces
    /// a writer at the same moment, and the registration channel needs
    /// its sender to go somewhere. Both belong to the caller, which
    /// pairs this with them and keeps the other ends.
    ///
    /// No scopes. A connection starts with none open, and every entry
    /// arrives through `registrations`.
    pub fn new(
        stream: SplitStream<Connection>,
        registrations: UnboundedReceiver<(u32, Option<u32>, Sender<Bytes>)>,
    ) -> Self {
        Router {
            stream,
            scopes: HashMap::new(),
            registrations,
        }
    }

    /// Read frames until the connection ends.
    ///
    /// Returns when the peer closes, when the transport errors, or
    /// when the stream simply stops. All three are the same event —
    /// there are no more frames — and every consumer learns it the same
    /// way: this is dropped, and with it every sender in it, so a
    /// receiver that was waiting sees its channel close.
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
    pub async fn run(mut self) {
        while let Some(received) = self.stream.next().await {
            let Ok(bytes) = received else { return };
            // Decoded for its header alone. The payload is borrowed and
            // then ignored; what gets forwarded is `bytes`, whole and
            // untouched.
            let Ok(frame) = ServerFrame::decode(&bytes) else { continue };
            match frame {
                // Not a scope's frame — it answers the connection, and
                // the connection is not routed.
                ServerFrame::Auth(_) => {}
                ServerFrame::Response { scope, .. } => {
                    if let Some(entry) = self.scope(scope) {
                        let _ = entry.sender.send(bytes).await;
                    }
                }
                // The scope is over, so everything under it goes: its
                // own sender and every channel that was open inside it.
                // Forwarded first, because the finish is what says the
                // stream ended rather than the connection.
                ServerFrame::ResponseFinish { scope } => {
                    if let Some(entry) = self.scope(scope) {
                        let _ = entry.sender.send(bytes).await;
                    }
                    self.scopes.remove(&scope);
                }
                // One frame and the channel is done — a request is the
                // whole of what arrives on it, and what follows goes
                // the other way, where this never looks.
                ServerFrame::ChannelRequest { scope, channel, .. } => {
                    if let Some(sender) = self.channel(scope, channel) {
                        let _ = sender.send(bytes).await;
                    }
                    self.close_channel(scope, channel);
                }
                ServerFrame::ChannelResponse { scope, channel, .. } => {
                    if let Some(sender) = self.channel(scope, channel) {
                        let _ = sender.send(bytes).await;
                    }
                }
                // As `ResponseFinish`, one level down: forward, then
                // drop the channel and leave the scope open.
                ServerFrame::ChannelResponseFinish { scope, channel } => {
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
    /// is otherwise unavoidable: a registration and the request that
    /// causes its frames travel by different routes, and the answer can
    /// beat the registration here. Draining on a miss closes that
    /// window — anything registered before the frame was read is found
    /// on the second look.
    ///
    /// A miss after the drain is a real miss. Nothing waits for a
    /// registration that has not been made, because nothing says one is
    /// coming.
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

    /// Drop a channel, and leave the scope it was in alone.
    ///
    /// A scope outlives its channels — it is the request, and they are
    /// the exchanges inside it — so a channel ending takes nothing else
    /// with it. A scope that is already gone took this with it.
    fn close_channel(&mut self, scope: u32, channel: u32) {
        if let Some(entry) = self.scopes.get_mut(&scope) {
            entry.channels.remove(&channel);
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
        while let Ok((scope, channel, sender)) = self.registrations.try_recv() {
            match channel {
                None => {
                    self.scopes.entry(scope).or_insert_with(|| Scope {
                        sender,
                        channels: HashMap::new(),
                    });
                }
                Some(channel) => {
                    let Some(entry) = self.scopes.get_mut(&scope) else {
                        continue;
                    };
                    entry.channels.entry(channel).or_insert(sender);
                }
            }
        }
    }
}

/// Where one scope's frames go.
///
/// Nested inside [`Router`]'s map rather than flattened into a
/// `(scope, channel)` key, because a scope ending ends everything
/// under it: one removal drops the responses and every channel at
/// once, where a flat map would have to be scanned for them.
#[derive(Debug)]
struct Scope {
    /// The answers to the request that opened the scope.
    ///
    /// Channel `0`, which the frame layer already treats as the
    /// scope's own — so it does not need a key in
    /// [`channels`](Self::channels) and does not get one.
    sender: Sender<Bytes>,
    /// The channels inside it, by number.
    ///
    /// One space, not two. A router reads frames travelling in ONE
    /// direction, so the channels it sees were all opened by the same
    /// side — the ambiguity that makes a bare channel number
    /// meaningless never arises here.
    channels: HashMap<u32, Sender<Bytes>>,
}
