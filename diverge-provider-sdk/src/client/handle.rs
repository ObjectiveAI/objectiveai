//! The other end of the read loop: what a caller actually holds.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bytes::Bytes;
use futures_util::stream::SplitSink;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::router::Registration;
use crate::connection::Connection;

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
    scopes: HashMap<u32, Scope>,
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
}

/// One open scope, as the side that opened it sees it.
///
/// Numbers and nothing else. Where the frames of this scope go is the
/// router's business and is not duplicated here; what is here is what
/// the router cannot know, which is what has been given out.
// As with `HandleInner`: nothing reads these until something mints.
#[allow(dead_code)]
#[derive(Debug)]
struct Scope {
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
