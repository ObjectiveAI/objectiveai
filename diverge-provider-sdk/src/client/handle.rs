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
/// A [`Router`](super::router::Router) is a task that runs; this is the
/// thing a caller keeps. Between them they own one socket, split down
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
/// # Shared, and the lock that costs
///
/// [`new`](Self::new) returns an [`Arc`], because one socket is what
/// every caller on this connection has to write to. Handing out clones
/// of the writer is not on offer — there is one, and it is the wire.
///
/// So the sink is behind a [`Mutex`]: shared ownership plus a
/// [`Sink`](futures_util::Sink) that needs `&mut` is exactly what a
/// mutex is for. It is [`tokio`]'s and not [`std`]'s, because the guard
/// is held across the `await` that writes the frame, and a `std` guard
/// is not [`Send`] across one.
///
/// What the lock buys is more than exclusion. A WebSocket forbids
/// interleaving the fragments of two messages, so one writer at a time
/// is not a policy — holding the lock for a whole frame is what makes
/// concurrent callers legal on one socket.
///
/// # Nothing is written yet
///
/// The fields are the whole of it. What goes on top — opening a scope,
/// opening a channel inside one, minting the numbers for either — is
/// not decided, and each of those decisions wants this to exist first.
///
/// One thing the fields decide in advance: `closed` is a receiver, and
/// receiving needs `&mut`, which an [`Arc`] does not give. Whatever
/// reads it either locks it as the sink is locked, or does not live
/// here at all.
// Nothing reads these yet, because nothing sends yet. The attribute
// goes when the methods do.
#[allow(dead_code)]
#[derive(Debug)]
pub struct Handle {
    /// The write half of the connection.
    ///
    /// Every frame this end sends goes out here, in order, one at a
    /// time — see above for why that is the wire's rule rather than a
    /// choice, and why the lock is an async one.
    ///
    /// Locked per frame, not per scope. A caller that held it across a
    /// stream of chunks would be the only caller on the connection for
    /// the duration, which is the stall this whole arrangement exists
    /// to avoid.
    sink: Mutex<SplitSink<Connection, Bytes>>,
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

impl Handle {
    /// Take the three parts, from whoever made all of them.
    ///
    /// The write half of a split [`Connection`], the sending end of the
    /// registration channel, and the receiving end of the closure
    /// channel — the exact complements of what a
    /// [`Router`](super::router::Router) is given. Neither is made
    /// here, because a split and a channel each produce two ends and
    /// only one of them belongs on this side.
    ///
    /// Pairing them correctly is the caller's to get right, and nothing
    /// here can check it. Two halves of different sockets, or channels
    /// crossed with another connection's, type-check exactly as well as
    /// the right ones.
    ///
    /// Comes back in an [`Arc`] rather than bare. Sharing is not an
    /// option a caller weighs later — a connection has one writer and
    /// everything that writes to it needs the same one — so it is not
    /// left to a caller to remember, and a `Handle` that was never
    /// shared is a connection with one user, which is the ordinary case
    /// and costs one refcount.
    pub fn new(
        sink: SplitSink<Connection, Bytes>,
        registrations: UnboundedSender<Registration>,
        closed: UnboundedReceiver<(u32, Option<u32>)>,
    ) -> Arc<Self> {
        Arc::new(Handle {
            sink: Mutex::new(sink),
            registrations,
            closed,
        })
    }
}
