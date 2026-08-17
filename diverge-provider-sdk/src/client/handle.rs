//! The other end of the read loop: what a caller actually holds.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bytes::Bytes;
use futures_util::SinkExt as _;
use futures_util::stream::SplitSink;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{
    Receiver, UnboundedReceiver, UnboundedSender, channel,
};

use super::router::Registration;
use crate::connection::Connection;
use crate::encode::{Encode, Writer};
use crate::endpoints::ClientRequest;
use crate::frame::client::ClientFrame;

/// How many frames a scope or a channel may be behind before the
/// router stops reading.
///
/// A guess, and the only number in this module that is one. It is deep
/// enough that a consumer doing ordinary work between reads never
/// stalls the connection, and shallow enough that a consumer that has
/// stopped reading is noticed while the memory it is holding is still
/// small. Nothing has measured it.
const CAPACITY: usize = 32;

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
    /// Holds the lock for the whole call, which is what makes it safe
    /// to call from anywhere: minting a number, claiming it, and
    /// writing the frame that spends it are one indivisible act. Two
    /// callers racing here would otherwise be two callers with the same
    /// scope.
    pub async fn send_request(&self, request: ClientRequest<'_>) -> Scope {
        self.0.lock().await.send_request(request).await
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
    scopes: HashMap<u32, ScopeState>,
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
    /// Nothing, here. A dead socket and a request that will not
    /// serialize both end the same way: the receivers in the returned
    /// [`Scope`] close without a finish frame, which is what a caller
    /// reads as "this scope is not happening". The scope number is not
    /// spent in the second case — the entry comes back out of the map
    /// before returning, because nothing on the wire ever named it.
    async fn send_request(&mut self, request: ClientRequest<'_>) -> Scope {
        self.take_back();
        let scope = self.mint_scope();
        let (response_sender, responses) = channel(CAPACITY);
        let (request_sender, requests) = channel(CAPACITY);
        let mut bytes = Vec::new();
        let encoded = ClientFrame::Request { scope, request }
            .encode(&mut Writer::new(&mut bytes));
        if encoded.is_err() {
            // Never sent, never registered, never named on the wire.
            // The senders drop here and the caller's receivers close
            // with them.
            self.scopes.remove(&scope);
            return Scope {
                scope,
                responses,
                requests,
            };
        }
        let _ = self.registrations.send(Registration::Scope {
            scope,
            response_sender,
            request_sender,
        });
        let _ = self.sink.send(bytes.into()).await;
        Scope {
            scope,
            responses,
            requests,
        }
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
                    if let Some(state) = self.scopes.get_mut(&scope) {
                        state.channels.remove(&channel);
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
                self.scopes.insert(
                    scope,
                    ScopeState {
                        channel_counter: 0,
                        channels: HashSet::new(),
                    },
                );
                return scope;
            }
        }
    }
}

/// A scope that has been opened, and the frames that will arrive in it.
///
/// What [`Handle::send_request`] gives back. The scope is open from the
/// moment this exists — the request has gone out, and the router
/// already knows where to put what comes back.
///
/// # Two streams, and why they are not one
///
/// [`responses`](Self::responses) is the answer to the request. It ends
/// at a finish frame and there is exactly one per scope.
///
/// [`requests`](Self::requests) is the server asking for something
/// inside this scope — serving an image, running a command, proxying
/// Postgres. There may be none, and there may be more of them than
/// answers.
///
/// They are separate because a channel number belongs to whoever opened
/// it: the server numbers its own from zero and so does this end, so
/// the two cannot share a stream without the numbers colliding.
///
/// # Reading is not optional
///
/// The receivers are bounded, and shallow. A caller that stops reading
/// one stops the router within a few dozen frames, and stopping the
/// router stops every scope on the connection — not just this one. Drop what you are not
/// going to read: a dropped receiver makes its sends fail, which the
/// router ignores and carries on.
#[derive(Debug)]
pub struct Scope {
    /// The scope's number, chosen by this end.
    ///
    /// It is in the header of every frame belonging to this scope, in
    /// both directions. Free for reuse once the scope closes, which is
    /// the [`Handle`]'s business rather than a caller's.
    pub scope: u32,
    /// The answer to the request, frame by frame.
    ///
    /// Whole frames, headers included, exactly as they came off the
    /// socket. Ends at the finish frame; the channel closing without
    /// one means the connection went first.
    pub responses: Receiver<Bytes>,
    /// The requests the server makes inside this scope.
    ///
    /// Whole frames again, and the channel number in each header is the
    /// SERVER's — it is what an answer has to quote to be understood.
    pub requests: Receiver<Bytes>,
}

/// What the opening side remembers about one open scope.
///
/// Numbers and nothing else. Where the frames of this scope go is the
/// router's business and is not duplicated here; what is here is what
/// the router cannot know, which is what has been given out.
// As with `HandleInner`: nothing reads these until something mints.
#[allow(dead_code)]
#[derive(Debug)]
struct ScopeState {
    /// Where the next channel number in this scope comes from.
    ///
    /// Per scope, not per connection, because a channel number is only
    /// ever read alongside the scope in the same frame header. Two
    /// scopes both using channel `0` are two different channels and
    /// neither has to know about the other.
    channel_counter: u32,
    /// The channels open inside it.
    ///
    /// A set, because there is nothing to store against them. What
    /// arrives on a channel goes to a receiver the router holds, so the
    /// only fact this side keeps is that the number is in use.
    channels: HashSet<u32>,
}
