//! The other end of the read loop: what a caller actually holds.

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
}

impl HandleInner {
    /// The same three parts, in the same order.
    ///
    /// Matching [`Handle::new`] because it is the one caller: the
    /// wrapper decides what a `Handle` is made of, and this decides
    /// nothing — there is no argument it could take that the public one
    /// does not already have to be given.
    fn new(
        sink: SplitSink<Connection, Bytes>,
        registrations: UnboundedSender<Registration>,
        closed: UnboundedReceiver<(u32, Option<u32>)>,
    ) -> Self {
        HandleInner {
            sink,
            registrations,
            closed,
        }
    }
}
