//! One scope a caller opened, and the frames that arrive in it.

use bytes::Bytes;
use tokio::sync::mpsc::Receiver;

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
/// The receivers are bounded, at whatever depth was asked for. A caller
/// that stops reading one stops the router that many frames later, and
/// stopping the router stops every scope on the connection — not just
/// this one. Drop what you are not going to read: a dropped receiver
/// makes its sends fail, which the router ignores and carries on.
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
    pub response_receiver: Receiver<Bytes>,
    /// The requests the server makes inside this scope.
    ///
    /// Whole frames again, and the channel number in each header is the
    /// SERVER's — it is what an answer has to quote to be understood.
    pub request_receiver: Receiver<Bytes>,
}
