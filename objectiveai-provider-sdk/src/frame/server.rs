//! Frames a server sends.
//!
//! A server answers the client's request, and may open channels of its
//! own to ask the client for things along the way. Both happen inside
//! the scope the client's request opened — a server never initiates
//! one.

/// A frame sent by a server.
///
/// [`Ack`](Self::Ack), [`Body`](Self::Body) and
/// [`Finish`](Self::Finish) carry no channel: they answer the client's
/// own request, which is always channel `0`. Only
/// [`Request`](Self::Request) names a channel, because it is the only
/// one that opens one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerFrame<'a> {
    /// Type `0` on channel `0`. Acknowledges the client's request and
    /// MINTS its scope — the first frame of the scope, and the only
    /// place the scope comes from.
    Ack {
        /// The newly minted scope.
        scope: u64,
    },
    /// Type `1` on channel `0`. One piece of the answer to the
    /// client's request.
    Body {
        /// The scope.
        scope: u64,
        /// The body bytes.
        payload: &'a [u8],
    },
    /// Type `2` on channel `0`. The scope is over. Nothing bearing it
    /// follows, on any channel.
    Finish {
        /// The scope.
        scope: u64,
    },
    /// Type `3` or above: a request to the client, opening a channel.
    ///
    /// The client answers on that same channel with its own ack, body
    /// and finish.
    Request {
        /// The scope this happens inside.
        scope: u64,
        /// A channel unique within the scope, minted here. Every
        /// server request gets its own, so several can be outstanding
        /// at once without their answers being confusable.
        channel: u64,
        /// Which kind of request. `3` or above; what each value means
        /// belongs to the protocol being carried, not to this layer.
        r#type: u8,
        /// The request bytes.
        payload: &'a [u8],
    },
}
