//! Frames a client sends.
//!
//! A client only ever does two things: open a scope, or answer a
//! request the server made inside one. It never opens a channel and
//! never sends a request of its own beyond the first.

/// A frame sent by a client.
///
/// The variants carry only what they can have, so the states that do
/// not exist cannot be built: every reply has both a scope and the
/// channel of the server request it answers, and
/// [`Request`](Self::Request) has neither, because neither exists
/// until the server answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientFrame<'a> {
    /// Type `0`. Acknowledges a server request; the exchange has
    /// begun.
    Ack {
        /// The scope the server minted.
        scope: u64,
        /// The channel of the server request being answered.
        channel: u64,
    },
    /// Type `1`. One piece of the answer. There may be any number,
    /// including none.
    Body {
        /// The scope the server minted.
        scope: u64,
        /// The channel of the server request being answered.
        channel: u64,
        /// The body bytes.
        payload: &'a [u8],
    },
    /// Type `2`. The answer is complete and the channel is closed.
    /// Nothing follows on it.
    Finish {
        /// The scope the server minted.
        scope: u64,
        /// The channel of the server request being answered.
        channel: u64,
    },
    /// Type `3`. A new request, opening a scope.
    ///
    /// Sent with no scope and no channel — the server mints the scope
    /// in its ack. A client has only this one kind of request, so it
    /// is the only value above `2` a client ever sends.
    Request {
        /// The request bytes.
        payload: &'a [u8],
    },
}
