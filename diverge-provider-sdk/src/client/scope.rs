//! One scope a caller opened, and the frames that arrive in it.

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

/// A scope that has been opened, and the frames that will arrive in it.
///
/// What [`Handle::send_request`](super::handle::Handle::send_request)
/// gives back. The scope is open from the moment this exists — the
/// request has gone out, and the router already knows where to put what
/// comes back.
///
/// # Two streams, and why they are not one
///
/// [`response_receiver`](Self::response_receiver) is the answer to the
/// request. It ends at a finish frame and there is exactly one per
/// scope.
///
/// [`request_receiver`](Self::request_receiver) is the server asking
/// for something inside this scope — serving an image, running a
/// command, proxying Postgres. There may be none, and there may be more
/// of them than answers.
///
/// They are separate because a channel number belongs to whoever opened
/// it: the server numbers its own from zero and so does this end, so
/// the two cannot share a stream without the numbers colliding.
///
/// # Reading is not optional
///
/// The receivers are unbounded, so a caller that stops reading stalls
/// nothing — not this scope, not the router, not the connection. What
/// it does instead is grow, at whatever rate the server is sending, and
/// nothing in this crate bounds that.
///
/// Which makes the rule the same and the reason different: drop what
/// you are not going to read. A dropped receiver makes the router's
/// sends fail, which it ignores and carries on, and frees what the
/// queue was holding.
#[derive(Debug)]
pub struct Scope {
    /// The scope's number, chosen by this end.
    ///
    /// It is in the header of every frame belonging to this scope, in
    /// both directions. Free for reuse once the scope closes, which is
    /// the [`Handle`](super::handle::Handle)'s business rather than a
    /// caller's.
    pub scope: u32,
    /// The answer to the request, frame by frame.
    ///
    /// Whole frames, headers included, exactly as they came off the
    /// socket. Ends at the finish frame; the channel closing without
    /// one means the connection went first.
    pub response_receiver: UnboundedReceiver<Bytes>,
    /// The requests the server makes inside this scope.
    ///
    /// Whole frames again, and the channel number in each header is the
    /// SERVER's — it is what an answer has to quote to be understood.
    pub request_receiver: UnboundedReceiver<Bytes>,
}
