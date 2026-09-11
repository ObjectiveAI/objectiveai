//! What a scope tells the session that the session cannot see.

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedSender;

/// What a scope tells the session it is doing.
///
/// One queue rather than two, which is not tidiness: a handle registers
/// a channel and later says its scope is over, and on separate queues
/// those can be applied the wrong way round. Here the order they were
/// sent in is the order they are applied in, and the hazard cannot be
/// expressed.
///
/// Merging is available on this side and not on the client's, and the
/// reason is worth keeping in view. A client's
/// [`Registration`](crate::client::registration::Registration) travels
/// one way and its closures travel the other, because a client's router
/// is what discovers that a stream ended. Here both kinds travel from
/// the scopes back to the session, so there is one direction and there
/// can be one queue.
///
/// Not public either, unlike that one, because a
/// [`Session`](super::session::Session) makes both ends of this itself.
/// There is nothing for a caller to wire up and so nothing for it to
/// name.
#[derive(Debug)]
pub(super) enum Notice {
    /// Somewhere to put an answer, arranged before it is asked for.
    ///
    /// Sent by the scope that is about to write the channel request,
    /// because it is the only party that knows an answer is coming.
    Register {
        /// The scope the channel is inside, as the client numbered it.
        scope: u32,
        /// The channel, chosen by this end.
        channel: u32,
        /// Where the client's answers on it go.
        response_sender: UnboundedSender<Bytes>,
    },
    /// Something is gone: a whole scope for [`None`], one channel
    /// inside it for [`Some`].
    ///
    /// Sent from a destructor in both cases, so it cannot be forgotten
    /// and covers abandonment as well as any deliberate ending.
    ///
    /// The scope case takes every channel under it, which is why the
    /// map is nested — a scope's end is one removal rather than a scan
    /// for everything that belonged to it.
    Closed(u32, Option<u32>),
}
