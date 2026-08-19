//! The other end of the read loop: what a caller actually holds.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bytes::Bytes;
use futures_util::SinkExt as _;
use futures_util::stream::SplitSink;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::channel::Channel;
use super::router::Registration;
use super::scope::Scope;
use crate::connection::Connection;
use crate::encode::{Encode, Writer};
use crate::frame::client::ClientFrame;

/// One connection's outbound half, and the way to be heard by the
/// inbound one.
///
/// Cheap to clone and shared by every clone — there is one socket
/// under here, so there is one of these however many exist.
///
/// # Why the lock is inside
///
/// What is inside is private and there is no way to get at it
/// unlocked. Which is the point: a caller that could hold the parts
/// without the lock could write half a frame and then wait, and the
/// next writer's bytes would land inside the first one's message. A
/// WebSocket forbids exactly that, and forbidding it in the type is
/// cheaper than documenting it.
///
/// The lock is [`tokio`]'s, because the guard is held across the
/// `await` that writes a frame and a [`std`] guard is not [`Send`]
/// across one.
///
/// # What it costs
///
/// One lock for the whole connection, so a caller waiting to write
/// waits behind a caller reading closures, and vice versa. Locking the
/// two fields separately would not, and it is not worth it: a closure
/// read is a queue pop, and the frame writes it would be interleaved
/// with are the thing that has to be exclusive anyway.
///
/// What must not happen is a guard held across a stream of frames. One
/// frame is the wire's unit of exclusion, and a caller holding the lock
/// for a stream of chunks is a caller who has taken the connection away
/// from everybody else until it finishes.
// Nothing reads it yet, because nothing locks it yet. The attribute
// goes when the methods do.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Handle(Arc<Mutex<HandleInner>>);

impl Handle {
    /// Take the three parts, from whoever made all of them.
    ///
    /// The write half of a split [`Connection`], the sending end of the
    /// registration channel, and the receiving end of the closure
    /// channel — the exact complements of what a
    /// [`Router`](super::router::Router) is given. None of them is made
    /// here, because a split and a channel each produce two ends and
    /// only one of each belongs on this side.
    ///
    /// Pairing them correctly is the caller's to get right, and nothing
    /// here can check it. Two halves of different sockets, or channels
    /// crossed with another connection's, type-check exactly as well as
    /// the right ones.
    pub fn new(
        sink: SplitSink<Connection, Bytes>,
        registrations: UnboundedSender<Registration>,
        closed: UnboundedReceiver<(u32, Option<u32>)>,
    ) -> Self {
        Handle(Arc::new(Mutex::new(HandleInner::new(
            sink,
            registrations,
            closed,
        ))))
    }

    /// Open a scope, and send the request that opens it.
    ///
    /// Comes back with the scope's number and the two receivers its
    /// frames will arrive on. See [`Scope`].
    ///
    /// The payload is written as given, tag and all. This layer does
    /// not know what a request says and does not need to — see
    /// [`ClientRequest`](crate::endpoints::ClientRequest) for the ten
    /// it could be, and [`endpoints`](crate::endpoints) for what each
    /// one means.
    ///
    /// Holds the lock for the whole call, which is what makes it safe
    /// to call from anywhere: minting a number, claiming it, and
    /// writing the frame that spends it are one indivisible act. Two
    /// callers racing here would otherwise be two callers with the same
    /// scope.
    ///
    /// # Choosing the capacities
    ///
    /// They belong to the caller because only the caller knows what it
    /// asked for. A volume listing answers once; a file read answers in
    /// chunks for as long as the file lasts. One number for both would
    /// be too deep for the first or too shallow for the second.
    ///
    /// Depth buys tolerance for a slow reader and costs memory while it
    /// goes unread — at most `capacity` frames of it per stream, and a
    /// frame is as large as whatever the provider chunked. Too shallow
    /// costs no frames at all: nothing is dropped, the router blocks on
    /// the full queue instead, and it blocks for every scope on the
    /// connection rather than only this one.
    ///
    /// Zero is not allowed and panics, which is
    /// [`tokio`](tokio::sync::mpsc::channel)'s rule rather than this
    /// one.
    pub async fn send_request(
        &self,
        payload: &[u8],
        response_capacity: usize,
        request_capacity: usize,
    ) -> Scope {
        self.0
            .lock()
            .await
            .send_request(payload, response_capacity, request_capacity)
            .await
    }

    /// Open a channel inside a scope, and send the request that opens
    /// it.
    ///
    /// `None` if that scope is not open. Which is a race a caller
    /// cannot always avoid — a scope ends when its finish frame
    /// arrives, and that can happen between reading the last response
    /// and asking for a channel — so it is an answer rather than a
    /// panic. Nothing was sent when it comes back.
    ///
    /// The payload is written as given, tag and all. This layer does
    /// not know what a channel request says; see
    /// [`endpoints`](crate::endpoints) for who does.
    ///
    /// # One receiver, not two
    ///
    /// A channel this end opened has one thing coming back on it, so
    /// [`Channel`] carries one receiver where [`Scope`] carries two.
    /// There is no nesting: what the server opens arrives on the
    /// scope's own [`request_receiver`](Scope::request_receiver),
    /// whichever channel of this end's it was prompted by.
    pub async fn send_channel_request(
        &self,
        scope: u32,
        payload: &[u8],
        response_capacity: usize,
    ) -> Option<Channel> {
        self.0
            .lock()
            .await
            .send_channel_request(scope, payload, response_capacity)
            .await
    }

    /// Answer, on a channel the server opened.
    ///
    /// The scope and the channel are quoted from the request being
    /// answered — both are in the header of the frame that arrived on
    /// [`Scope::request_receiver`], and the channel is in the SERVER's
    /// numbering, which is why it has to be quoted rather than chosen.
    ///
    /// Any number of these, including none, and then exactly one
    /// [`send_channel_response_finish`](Self::send_channel_response_finish).
    /// Nothing else ends the answer: a slow one is not a finished one,
    /// and there is no timeout anywhere in this protocol.
    ///
    /// # Nothing comes back
    ///
    /// So there is nothing to return and nothing to register. This end
    /// did not open the channel, and a channel carries one side's
    /// answer to the other side's request — the request already
    /// arrived, and this is the answer going the other way.
    ///
    /// Nothing is checked either. This end tracks the numbers IT mints,
    /// and the server's are not among them, so a misquoted channel is a
    /// frame the server discards and this end never hears about.
    pub async fn send_channel_response(
        &self,
        scope: u32,
        channel: u32,
        payload: &[u8],
    ) {
        self.0
            .lock()
            .await
            .send_channel_response(scope, channel, payload)
            .await;
    }

    /// End an answer on a channel the server opened.
    ///
    /// One frame, no payload, and the channel is over. The far side
    /// learns the answer is complete here and nowhere else — the same
    /// rule this end relies on when it reads
    /// [`Scope::response_receiver`].
    ///
    /// Send it even for an answer that carried nothing. An empty answer
    /// and an answer still coming are the same thing until this arrives.
    pub async fn send_channel_response_finish(&self, scope: u32, channel: u32) {
        self.0
            .lock()
            .await
            .send_channel_response_finish(scope, channel)
            .await;
    }
}

/// What a [`Handle`] is a handle to.
///
/// A [`Router`](super::router::Router) is a task that runs; this is
/// what the callers share. Between them they own one socket, split down
/// the middle: the router reads and this writes, and the two channels
/// here are how the writing side arranges to be told what the reading
/// side found.
///
/// # Why the pair is not one type
///
/// Because a [`Sink`](futures_util::Sink) needs `&mut`, and a read loop
/// needs to run forever. One type holding both would have to be locked
/// for the duration of the loop, and nothing could write while it ran —
/// see [`connection`](crate::connection) for why that `&mut` is a wire
/// requirement and not an implementation detail.
///
/// Splitting is what makes both possible at once, and splitting is what
/// makes the channels necessary: the halves cannot see each other's
/// state, so anything one needs from the other travels as a message.
///
/// # Why it is behind a lock and not an [`Arc`] alone
///
/// Two of the three fields need `&mut` to be used at all: writing to a
/// [`Sink`](futures_util::Sink) needs one, and so does receiving on an
/// [`UnboundedReceiver`]. A shared reference gives neither, so sharing
/// without a lock would be sharing something nobody can use.
///
/// # What it knows that the router does not
///
/// Which numbers are taken. The router has a map of the same scopes and
/// it is not the same map: the router's says where frames GO, and this
/// one says which numbers are SPOKEN FOR. Only this side can keep the
/// second, because only this side mints them — the client chooses every
/// scope and every channel it opens, which is what makes one minter
/// enough and collisions this end's fault when they happen.
///
/// # Nothing is written yet
///
/// The fields are the whole of it. What goes on top — opening a scope,
/// opening a channel inside one, minting the numbers for either — is
/// not decided, and each of those decisions wants this to exist first.
// Nothing reads these yet, because nothing sends yet. The attribute
// goes when the methods do.
#[allow(dead_code)]
#[derive(Debug)]
struct HandleInner {
    /// The write half of the connection.
    ///
    /// Every frame this end sends goes out here, in order, one at a
    /// time. Which is not a queue discipline anybody chose: a WebSocket
    /// forbids interleaving the fragments of two messages, so one
    /// writer at a time is the wire's rule.
    ///
    /// Which is what sets the granularity of the lock around all of
    /// this: exclusive for one frame, never for a stream of them. A
    /// caller holding it across a stream of chunks would be the
    /// connection's only user until it finished.
    sink: SplitSink<Connection, Bytes>,
    /// Where to say that frames are coming, before sending the request
    /// that causes them.
    ///
    /// The order matters and it is this way round. A registration that
    /// went out after its request would be racing the answer, and the
    /// router would have nowhere to put a frame that beat it — see
    /// [`Registration`].
    ///
    /// Unbounded, so that arranging never blocks. A registration that
    /// waited behind a full queue would be a request whose answers
    /// arrive before anywhere exists to put them.
    registrations: UnboundedSender<Registration>,
    /// Where the router says an entry is gone.
    ///
    /// `(scope, channel)`, `None` for a whole scope. It arrives after
    /// the finish frame it followed, and it is not that frame's
    /// duplicate: the finish tells whoever was reading that stream that
    /// the stream is over, and this tells whoever is minting numbers
    /// that the number is free.
    closed: UnboundedReceiver<(u32, Option<u32>)>,
    /// Where the next scope number comes from.
    ///
    /// Nobody else mints one, on this connection or anywhere: a scope
    /// belongs to the client that opened it, and the numbers are only
    /// ever compared with the ones in [`scopes`](Self::scopes).
    ///
    /// What happens at the top is not decided. A `u32` that wraps
    /// arrives back at numbers that may still be open, and the counter
    /// alone cannot tell — the map beside it can.
    scope_counter: u32,
    /// The scopes this end has open, by number.
    ///
    /// Not the router's map, though it holds the same numbers. That one
    /// says where a frame goes; this one says what has been handed out
    /// and not yet taken back, which is the question a minter asks and
    /// a router never does.
    ///
    /// An entry leaves when the router says its scope closed — that
    /// message exists for this map.
    ///
    /// `(channel_counter, channels)`: where the next channel number in
    /// that scope comes from, and which ones are open. The counter is
    /// per scope because a channel number is only ever read alongside
    /// the scope in the same frame header — two scopes both using
    /// channel `1` are two different channels, and neither has to know
    /// about the other.
    ///
    /// A set for the channels, because there is nothing to store
    /// against them. What arrives on one goes to a receiver the router
    /// holds, so the only fact this side keeps is that the number is in
    /// use.
    scopes: HashMap<u32, (u32, HashSet<u32>)>,
    /// Where a frame is built, before it is handed to the socket.
    ///
    /// One buffer for the whole connection, which the lock is what
    /// makes possible: a frame is encoded and copied out before the
    /// guard is released, so no two builders ever overlap in it. It
    /// grows to the largest frame this connection has sent and stops.
    ///
    /// The copy at the end is not avoidable. A
    /// [`Sink`](futures_util::Sink) of [`Bytes`] takes ownership of
    /// what it sends, and this buffer is not giving up ownership — that
    /// is the whole point of it.
    ///
    /// What it saves is the growth. A fresh [`Vec`] per frame
    /// reallocates its way up from nothing every time, copying at each
    /// step; this reallocates once ever and pays one exact-size copy
    /// per frame.
    buffer: Vec<u8>,
}

impl HandleInner {
    /// The same three parts, in the same order.
    ///
    /// Matching [`Handle::new`] because it is the one caller: the
    /// wrapper decides what a `Handle` is made of, and this decides
    /// nothing — there is no argument it could take that the public one
    /// does not already have to be given.
    ///
    /// The bookkeeping is not among them. A connection starts with no
    /// scopes open and its first at `0`, and there is no other answer a
    /// caller could give: a number already handed out is a number this
    /// end handed out, and this end is what is being made.
    fn new(
        sink: SplitSink<Connection, Bytes>,
        registrations: UnboundedSender<Registration>,
        closed: UnboundedReceiver<(u32, Option<u32>)>,
    ) -> Self {
        HandleInner {
            sink,
            registrations,
            closed,
            scope_counter: 0,
            scopes: HashMap::new(),
            buffer: Vec::new(),
        }
    }

    /// Open a scope, and send the request that opens it.
    ///
    /// In order: take back what has closed, mint a number nothing else
    /// is using, say where its frames go, and send the frame. The
    /// middle two are the ones that cannot be reordered — a request
    /// that went out before its registration would be a request whose
    /// answer has nowhere to land.
    ///
    /// # What a failed send does
    ///
    /// Nothing, here. The receivers in the returned [`Scope`] close
    /// without a finish frame, which is what a caller reads as "this
    /// scope is not happening".
    ///
    /// There is no failed encode. A frame is a header and a payload
    /// this crate never looks at, so encoding one is
    /// [`Infallible`](std::convert::Infallible) and says so in the
    /// type.
    async fn send_request(
        &mut self,
        payload: &[u8],
        response_capacity: usize,
        request_capacity: usize,
    ) -> Scope {
        self.take_back();
        let scope = self.mint_scope();
        let (response_sender, response_receiver) =
            mpsc::channel(response_capacity);
        let (request_sender, request_receiver) =
            mpsc::channel(request_capacity);
        let _ = self.registrations.send(Registration::Scope {
            scope,
            response_sender,
            request_sender,
        });
        self.send_frame(ClientFrame::Request { scope, payload }).await;
        Scope {
            scope,
            response_receiver,
            request_receiver,
        }
    }

    /// Open a channel inside a scope, and send the request that opens
    /// it.
    ///
    /// The same order as [`send_request`](Self::send_request) and for
    /// the same reasons: take back what has closed, mint, register,
    /// send. What differs is that the scope is given rather than minted
    /// — so it can be missing, which is the one failure this can report
    /// before touching the wire.
    ///
    /// It is the only failure. Encoding a frame cannot fail — a header
    /// and a payload nobody reads — so a scope that is open is a
    /// request that goes out.
    async fn send_channel_request(
        &mut self,
        scope: u32,
        payload: &[u8],
        response_capacity: usize,
    ) -> Option<Channel> {
        self.take_back();
        let channel = mint_channel(self.scopes.get_mut(&scope)?);
        let (response_sender, responses) = mpsc::channel(response_capacity);
        let _ = self.registrations.send(Registration::Channel {
            scope,
            channel,
            response_sender,
        });
        self.send_frame(ClientFrame::ChannelRequest {
            scope,
            channel,
            payload,
        })
        .await;
        Some(Channel { channel, responses })
    }

    /// Answer, on a channel the server opened.
    ///
    /// One frame and nothing else — no minting, no registration, no
    /// bookkeeping. A channel this end did not open leaves nothing here
    /// to keep, and the numbers are the server's; see
    /// [`Handle::send_channel_response`] for why neither is checked.
    async fn send_channel_response(
        &mut self,
        scope: u32,
        channel: u32,
        payload: &[u8],
    ) {
        self.send_frame(ClientFrame::ChannelResponse {
            scope,
            channel,
            payload,
        })
        .await;
    }

    /// End an answer on a channel the server opened.
    ///
    /// As above, with no payload. What makes it the last frame is the
    /// type, which the far side reads off the header.
    async fn send_channel_response_finish(&mut self, scope: u32, channel: u32) {
        self.send_frame(ClientFrame::ChannelResponseFinish { scope, channel })
            .await;
    }

    /// Build one frame and write it.
    ///
    /// Every frame this end sends goes through here, which is what
    /// keeps the buffer's protocol in one place: clear, encode, copy
    /// out, write. Cleared and not merely reused because a
    /// [`Writer`] appends from wherever the buffer already ends, so an
    /// uncleared one would send the last frame with this one glued to
    /// its back.
    ///
    /// A failed write is dropped. It means the connection is gone, and
    /// every receiver on it is about to close on its own — there is
    /// nothing this could tell a caller that the caller is not about to
    /// find out.
    async fn send_frame(&mut self, frame: ClientFrame<'_>) {
        self.buffer.clear();
        frame
            .encode(&mut Writer::new(&mut self.buffer))
            .unwrap_or_else(|error| match error {});
        let bytes = Bytes::copy_from_slice(&self.buffer);
        let _ = self.sink.send(bytes).await;
    }

    /// Drop every scope and channel the router says is finished.
    ///
    /// The queue is emptied rather than sampled, and it is emptied
    /// before minting, because what it holds is exactly the numbers
    /// that are free again. A mint that ran first would skip over them
    /// and hand out a larger number for no reason.
    ///
    /// A closure for a scope that is already gone, or a channel inside
    /// one, is nothing to do. Both mean the same thing, which is that
    /// the number is not in use.
    fn take_back(&mut self) {
        while let Ok((scope, channel)) = self.closed.try_recv() {
            match channel {
                None => {
                    self.scopes.remove(&scope);
                }
                Some(channel) => {
                    if let Some((_, channels)) = self.scopes.get_mut(&scope)
                    {
                        channels.remove(&channel);
                    }
                }
            }
        }
    }

    /// Take the next free scope number, and claim it.
    ///
    /// Counts up and steps over anything open, wrapping at the top
    /// rather than stopping there. Which is why the counter is not
    /// enough on its own: after a wrap the numbers below it may still
    /// be in use, and only the map knows.
    ///
    /// It claims as it returns, because a number that was minted and
    /// not recorded is a number the next call will mint again.
    ///
    /// Never returns if all four billion are open. That is not a case
    /// worth handling — it is a client holding four billion scopes,
    /// which the map itself could not fit in memory.
    fn mint_scope(&mut self) -> u32 {
        loop {
            self.scope_counter = self.scope_counter.wrapping_add(1);
            let scope = self.scope_counter;
            if !self.scopes.contains_key(&scope) {
                self.scopes.insert(scope, (0, HashSet::new()));
                return scope;
            }
        }
    }
}

/// Take the next free channel number in one scope, and claim it.
///
/// [`mint_scope`](HandleInner::mint_scope) one level down, with the
/// same wrap for the same reason. What is shorter here is the claim: a
/// [`HashSet`] insert answers "was it free" and takes it in one move,
/// where a map has to be asked and then told.
///
/// A free function because a scope's state is a
/// `(counter, channels)` pair rather than a type — two numbers with
/// nothing to hold them together but the entry they live in.
fn mint_channel((counter, channels): &mut (u32, HashSet<u32>)) -> u32 {
    loop {
        *counter = counter.wrapping_add(1);
        if channels.insert(*counter) {
            return *counter;
        }
    }
}
