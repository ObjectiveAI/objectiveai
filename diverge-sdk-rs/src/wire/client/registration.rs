//! Somewhere for a scope's frames to go, arranged before they arrive.

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedSender;

/// Somewhere to put frames, arranged before any of them arrive.
///
/// Sent to a [`Router`](super::router::Router), rather than installed
/// by one, because the party that knows a scope is coming is the party
/// about to open it, and that is not the router. It races with the
/// frames it is for, which a [`Router`](super::router::Router) handles
/// by draining this queue whenever a lookup misses.
///
/// # Why a scope brings two senders and a channel brings one
///
/// Two things arrive inside a scope that cannot be told apart
/// afterwards: the answers to the request, and the channels the server
/// opens. Both are the server talking, both are inside one scope, and
/// the client that opened it is the only party that ever knows both are
/// wanted. So both are registered together, and there is no way to
/// register half a scope.
///
/// A channel brings one, because a channel this end opened has one
/// thing coming back on it.
#[derive(Debug)]
pub enum Registration {
    /// Open a scope. Nothing routes into one until this arrives.
    Scope {
        /// The scope, chosen by whoever is about to request it.
        scope: u32,
        /// Where the answers on channel `0` go.
        response_sender: UnboundedSender<Bytes>,
        /// Where the server's own channel requests go, whatever it
        /// numbers them.
        request_sender: UnboundedSender<Bytes>,
    },
    /// Open a channel inside a scope that is already registered.
    ///
    /// Discarded if it is not. A channel entry lives inside a scope's
    /// entry, and a [`Router`](super::router::Router) will not invent
    /// the scope to put it in.
    Channel {
        /// The scope it is inside.
        scope: u32,
        /// The channel, chosen by whoever is about to request it.
        channel: u32,
        /// Where the server's answers on it go.
        response_sender: UnboundedSender<Bytes>,
    },
}
