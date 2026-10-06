//! The daemon the container speaks to, answering its frames.

use std::future::Future;

use futures_util::Stream;

use crate::shared::containers::daemon::{request, response};
use crate::shared::error::Error;

/// What answers a container's daemon connection, one client frame at
/// a time: the daemon's session for the container, which the caller
/// keeps for the container's life.
///
/// Each frame the container's program sends on its proxy's `/daemon`
/// reaches this as one [`request::Owned`] — a request opening a scope,
/// a channel request on one, a channel response or its finish — and
/// what this hands back is the stream of server frames that answer
/// it, as [`daemon`](crate::shared::containers::daemon) states: every
/// frame of the scope for a request, every frame of the channel for a
/// channel request, nothing for the rest. The stream's end is the
/// channel's finish. An `Err` in it is the frame the session could not
/// serve at all — the session gone, a scope it does not have — and the
/// last thing the stream yields.
///
/// # The session is the caller's
///
/// Nothing here mints scopes or channels: the program does, as a
/// client, and the numbers ride inside the frames. This is handed each
/// frame as it is, and the session it feeds is served for the
/// container's [`account`](crate::daemon::create::Inner::account),
/// which the caller knows from the run.
///
/// # Every frame is its own stream
///
/// Frames arrive on channels of their own, at once, and are answered
/// on tasks of their own, so a long scope — a watch, an upload — does
/// not hold up the frame after it.
pub trait Daemon: Send + Sync {
    /// The server frames that answer one client frame, in order, then
    /// the end; or, last, the error.
    type Frames: Stream<Item = Result<response::Owned, Error>> + Send + 'static;

    /// Feed one client frame to the container's session, and hand
    /// back what answers it.
    fn frame(&self, frame: request::Owned) -> impl Future<Output = Self::Frames> + Send;
}
