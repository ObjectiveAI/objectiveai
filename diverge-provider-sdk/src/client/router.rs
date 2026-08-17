//! The read loop: frames in, and off to whoever is waiting.

use std::collections::HashMap;

use bytes::Bytes;
use futures_util::stream::SplitStream;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};

use crate::connection::Connection;

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
/// # What is not settled
///
/// **What a full queue means.** The senders are bounded, which
/// settles that a slow consumer cannot grow memory without limit on a
/// protocol that streams image layers. It does not settle what this
/// does when one fills: waiting stalls every other scope on the
/// connection, and not waiting drops a frame from a stream that has no
/// way to say it lost one.
///
/// **When entries leave.** A finish should take one out. A consumer
/// that walked away only surfaces when a send to it fails, which is
/// never if no further frame arrives — so a scope nobody is listening
/// to sits here until the connection ends.
// As above: the fields exist for the loop that is not written.
#[allow(dead_code)]
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

/// Where one scope's frames go.
///
/// Nested inside [`Router`]'s map rather than flattened into a
/// `(scope, channel)` key, because a scope ending ends everything
/// under it: one removal drops the responses and every channel at
/// once, where a flat map would have to be scanned for them.
// Nothing reads these yet, because nothing routes yet. The attribute
// goes when the loop does.
#[allow(dead_code)]
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
