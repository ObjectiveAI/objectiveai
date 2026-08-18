//! The read loop: frames in, and off to whoever is waiting.

use std::collections::HashMap;

use bytes::Bytes;
use futures_util::StreamExt as _;
use futures_util::stream::SplitStream;
use tokio::sync::mpsc::{
    self, Receiver, Sender, UnboundedReceiver, UnboundedSender,
};

use crate::connection::Connection;
use crate::frame::client::ClientFrame;

/// One connection's inbound half.
///
/// Reads frames, looks up where each belongs, and forwards it. The
/// mirror of [`client::router::Router`](crate::client::router::Router),
/// and the same in every respect the two halves share: whole frames go
/// on untouched, header included; one hop from here to the consumer;
/// bounded senders, so a consumer that stops reading stops the
/// connection rather than losing a frame.
///
/// # What is not mirrored
///
/// **A scope arrives rather than being registered.** Only a client
/// opens one, so this end cannot be told in advance where a scope's
/// frames go — the frame that opens it is the first anyone hears of
/// it. So this makes the entry itself and hands out a [`Request`],
/// where the client's router waits to be told.
///
/// Which turns the lookup around. On the client's side a miss means
/// "not registered YET", and draining the queue is worth a try. Here a
/// miss on a scope is final: nothing in the queue creates one, because
/// only a frame does.
///
/// **A scope ends from this end.** No client frame closes a scope —
/// the server's own `ResponseFinish` does — so the read loop never
/// sees it happen and has to be told. That is what [`Sent`] is: the
/// writing half saying what it wrote, in the order it wrote it.
///
/// **Closures only ever name a channel.** A scope this end closed is a
/// scope this end already knows about, so there is nothing to report
/// back — which is why a closure out of here carries `(scope,
/// channel)` and not the client router's `(scope, Option<u32>)`.
///
/// # Auth is dropped on the floor
///
/// And that is a gap, not a decision. A credential belongs to the
/// connection, this reads the connection, and there is nowhere for it
/// to go — no channel out of here carries one, so the frame is
/// discarded like a type nobody defines. Whatever ends up deciding
/// what a credential means will need a path through here, and does not
/// have one.
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
    /// Where a newly opened scope goes.
    ///
    /// One per client request, in arrival order, and the only way
    /// anything on this side learns a scope exists.
    ///
    /// Bounded, and a full queue stops the loop — the same trade as
    /// every other queue here, for a stronger reason: a dropped request
    /// is a scope the client is waiting on and nobody is serving, and
    /// the client would wait forever, because a stream ends at its
    /// finish frame and no finish would ever come.
    requests: Sender<Request>,
    /// How deep to make a scope's channel-request stream.
    ///
    /// Chosen here because there is nobody to ask: the scope does not
    /// exist until this makes it, and the party that will serve it has
    /// not seen it yet. One number for every scope on the connection,
    /// where a client picks one per request.
    request_capacity: usize,
    /// What the writing half has sent.
    ///
    /// Two things, and both change this map — see [`Sent`]. One queue
    /// rather than two, so they land in the order they were sent.
    ///
    /// Unbounded, for the same reason the client's registrations are: a
    /// writer must not block to say what it just did, least of all
    /// behind a queue that this loop is the one draining.
    sent: UnboundedReceiver<Sent>,
    /// Where this says a channel is finished.
    ///
    /// `(scope, channel)`, in this end's numbering, sent when the
    /// client's answer on a channel this end opened runs out. It frees
    /// the number for reuse; the receiver closing says the same thing
    /// to whoever was reading the answer.
    ///
    /// Unbounded, because it is sent from inside the loop that would
    /// have to drain it.
    closed: UnboundedSender<(u32, u32)>,
}

impl Router {
    /// Take the read half, and both ends that connect it to the writer.
    ///
    /// The read half of a split [`Connection`], the sending end of the
    /// request channel, the receiving end of the [`Sent`] channel, and
    /// the sending end of the closure channel. None is made here: each
    /// has another end, and the other ends belong together elsewhere.
    ///
    /// `request_capacity` is the depth of every scope's channel-request
    /// stream — one number for the connection, because the party that
    /// will serve a scope has not seen it when this has to make it. A
    /// client picks one per request instead.
    ///
    /// No scopes. A connection starts with none open, and every entry
    /// arrives as a frame.
    pub fn new(
        stream: SplitStream<Connection>,
        requests: Sender<Request>,
        request_capacity: usize,
        sent: UnboundedReceiver<Sent>,
        closed: UnboundedSender<(u32, u32)>,
    ) -> Self {
        Router {
            stream,
            scopes: HashMap::new(),
            requests,
            request_capacity,
            sent,
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
    /// A sixth is a scope-opening request for a scope that is already
    /// open. It is the one discard that is a client's mistake rather
    /// than a race, and the only one this end could have been wrong
    /// about — so it is decided only after draining what the writer has
    /// said, in case the scope has just been finished.
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
                ClientFrame::Request { scope, .. } => {
                    self.open(scope, bytes).await;
                }
                // The scope's stream, not a channel's. The channel
                // number in the header is the CLIENT's, and the map
                // holds this end's — see `request_sender`.
                //
                // Nothing is opened here that this has to remember. The
                // answer travels the other way, and what remembers it
                // is whatever writes it.
                ClientFrame::ChannelRequest { scope, .. } => {
                    // No drain on a miss: nothing in the queue creates
                    // a scope, so a scope that is absent is absent.
                    if let Some(entry) = self.scopes.get_mut(&scope) {
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

    /// Open a scope, and hand it to whoever is serving.
    ///
    /// The frame goes on whole, as every other frame does, inside a
    /// [`Request`] that also carries the stream the scope's channel
    /// requests will arrive on. Both have to exist before this returns:
    /// a channel request for this scope may be the very next frame.
    ///
    /// # A scope that is already open
    ///
    /// Discarded, and the client is the party that could have prevented
    /// it. Two live scopes with one number are indistinguishable in
    /// every frame that follows, and the alternative — replacing the
    /// entry — would silently strand whoever is serving the first one.
    ///
    /// The queue is drained before deciding that, and that is the whole
    /// reason the check is not a bare lookup: a scope this end finished
    /// is a number the client may legitimately reuse, and the
    /// [`ResponseFinish`](Sent::ResponseFinish) saying so may still be
    /// waiting. Draining first is the difference between reuse and
    /// collision.
    ///
    /// # Nobody serving
    ///
    /// If the request cannot be handed over, the entry comes back out.
    /// Keeping it would be keeping a scope with nowhere to deliver, and
    /// every frame that followed would be routed into a channel nobody
    /// holds the other end of.
    async fn open(&mut self, scope: u32, bytes: Bytes) {
        if self.scopes.contains_key(&scope) {
            self.drain();
            if self.scopes.contains_key(&scope) {
                return;
            }
        }
        let (request_sender, request_receiver) =
            mpsc::channel(self.request_capacity);
        self.scopes.insert(
            scope,
            Scope {
                request_sender,
                channels: HashMap::new(),
            },
        );
        let request = Request {
            scope,
            request: bytes,
            request_receiver,
        };
        if self.requests.send(request).await.is_err() {
            self.scopes.remove(&scope);
        }
    }

    /// Find a channel inside a scope, draining first if it is not
    /// there.
    ///
    /// The retry is for a race that is otherwise unavoidable, and it is
    /// the client router's race with the ends swapped: this side opens
    /// a channel by writing a frame and saying so on [`sent`](Self::sent),
    /// and those travel separately. The answer can arrive before the
    /// word does.
    ///
    /// A miss after the drain is a real miss — a channel this end never
    /// opened, or one whose answer already finished.
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
    /// Only on a real removal. A finish for a channel that is already
    /// gone announces nothing, so that whoever is counting what it
    /// opened against what it closed is never told twice.
    ///
    /// The scope stays. It ends when this end finishes it and not
    /// before — see [`Sent::ResponseFinish`].
    fn close_channel(&mut self, scope: u32, channel: u32) {
        let Some(entry) = self.scopes.get_mut(&scope) else {
            return;
        };
        if entry.channels.remove(&channel).is_some() {
            let _ = self.closed.send((scope, channel));
        }
    }

    /// Apply everything the writing half has said, in order.
    ///
    /// Emptied rather than sampled, on a miss and before deciding a
    /// scope number is taken. Two kinds arrive and they pull in
    /// opposite directions — one adds a channel, the other removes a
    /// whole scope — which is exactly why they share a queue: applied
    /// out of order, a close and a reopen of the same number are the
    /// same two messages with the opposite meaning.
    ///
    /// A channel for a scope that is gone is discarded. It is not a
    /// mistake here the way it is on the client's side: the scope may
    /// have been finished by this end between the writer registering
    /// and this reading it, which is a race nobody could have avoided
    /// and nothing to do about.
    ///
    /// A duplicate channel keeps the entry it already had, on the same
    /// argument as everywhere else — replacing it would redirect a
    /// stream somebody is still reading.
    fn drain(&mut self) {
        while let Ok(sent) = self.sent.try_recv() {
            match sent {
                Sent::ChannelRequest {
                    scope,
                    channel,
                    response_sender,
                } => {
                    let Some(entry) = self.scopes.get_mut(&scope) else {
                        continue;
                    };
                    entry.channels.entry(channel).or_insert(response_sender);
                }
                Sent::ResponseFinish { scope } => {
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
    /// The number is not lost: it is in the header of the frame that
    /// goes down this, whole. What this end does with it is answer on
    /// it, which is a write, and writes are not a router's business.
    ///
    /// Nothing is ever removed for one of these. A request is the whole
    /// of what arrives — one frame, opening a channel whose remaining
    /// traffic travels the other way — so there is no entry to keep and
    /// none to take away.
    request_sender: Sender<Bytes>,
    /// The channels this SERVER opened, by number, and what the client
    /// says back on each.
    ///
    /// One space, because one opener: everything in here was numbered
    /// by this end. What the client numbers goes to
    /// [`request_sender`](Self::request_sender) instead, which is what
    /// keeps that true.
    channels: HashMap<u32, Sender<Bytes>>,
}

/// A scope a client opened, and what will arrive inside it.
///
/// What a [`Router`] hands out, once per scope-opening request. The
/// scope is open from the moment this exists, and stays open until this
/// end finishes it — nothing a client sends ends one.
///
/// # Reading is not optional
///
/// [`request_receiver`](Self::request_receiver) is bounded. Whoever
/// holds this either reads it or drops it: a full queue stops the
/// router, and a stopped router stops every scope on the connection.
/// Dropping is safe — a send to a dropped receiver fails, and the
/// router ignores that and carries on.
#[derive(Debug)]
pub struct Request {
    /// The scope's number, chosen by the client.
    ///
    /// Every frame belonging to this scope carries it, in both
    /// directions, and every answer has to quote it.
    pub scope: u32,
    /// The frame that opened the scope, whole.
    ///
    /// Header included, like everything else this router forwards, so
    /// the payload starts at
    /// [`HEADER_LEN`](crate::frame::HEADER_LEN). What the payload means
    /// is [`ClientRequest`](crate::endpoints::ClientRequest), and
    /// decoding it is this holder's second step — decoding cannot fail,
    /// so a request nobody can name still arrives as one and is still
    /// answerable in the scope it opened.
    pub request: Bytes,
    /// The channel requests the client makes inside this scope.
    ///
    /// Whole frames, and the channel number in each header is the
    /// CLIENT's — it is what an answer has to quote to be understood.
    /// There may be none, and there is no announcement when there will
    /// be no more: they stop when the scope does.
    pub request_receiver: Receiver<Bytes>,
}

/// What the writing half has sent, so that the reading half can keep
/// up.
///
/// Two frames change what this connection's map should hold, and
/// neither is one the read loop can see, because this end is the one
/// that sends them. So they are announced.
///
/// One enum on one queue, because they must be applied in the order
/// they were sent. A scope closed and a channel opened inside it are
/// two edits to the same entry, and the wrong order leaves a channel in
/// a scope that is over or drops a scope that has just been reused.
#[derive(Debug)]
pub enum Sent {
    /// A channel request went out. Its answers go here.
    ///
    /// Say it BEFORE writing the frame. The client may answer at once,
    /// and an announcement that lost that race is an answer with
    /// nowhere to go — the drain closes the window, but only for
    /// something already in the queue.
    ChannelRequest {
        /// The scope it is inside.
        scope: u32,
        /// The channel, chosen by this end.
        channel: u32,
        /// Where the client's answers on it go.
        response_sender: Sender<Bytes>,
    },
    /// A response finish went out, so the scope is over.
    ///
    /// Everything under it goes: the channel-request stream, and every
    /// channel this end still had open inside it. There is no closure
    /// reported for any of them, because the party that would be told
    /// is the party that just said this.
    ResponseFinish {
        /// The scope that is over.
        scope: u32,
    },
}
