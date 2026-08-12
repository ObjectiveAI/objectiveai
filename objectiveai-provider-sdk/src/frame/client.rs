//! Frames a client sends.
//!
//! A client opens a scope, answers requests the server makes inside
//! one, and — if it was the side that dialled — authenticates. It
//! never opens a channel and never sends a request of its own beyond
//! the first.

use super::FrameError;

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
        scope: u32,
        /// The channel of the server request being answered.
        channel: u32,
    },
    /// Type `1`. One piece of the answer. There may be any number,
    /// including none.
    Body {
        /// The scope the server minted.
        scope: u32,
        /// The channel of the server request being answered.
        channel: u32,
        /// The body bytes.
        payload: &'a [u8],
    },
    /// Type `2`. The answer is complete and the channel is closed.
    /// Nothing follows on it.
    Finish {
        /// The scope the server minted.
        scope: u32,
        /// The channel of the server request being answered.
        channel: u32,
    },
    /// Type `3`. The opening frame of a connection, sent by whichever
    /// side dialled.
    ///
    /// No scope and no channel: nothing has been established yet, and
    /// this is what establishes it. The payload is arbitrary — what
    /// counts as a credential is not this layer's business.
    AuthRequest {
        /// The credential, in whatever form the two ends agreed.
        payload: &'a [u8],
    },
    /// Type `4`. The answer to an [`AuthRequest`](Self::AuthRequest),
    /// sent by whichever side accepted the connection.
    ///
    /// No scope and no channel, for the same reason. The payload
    /// carries the verdict and anything that comes with it.
    AuthResponse {
        /// The verdict, in whatever form the two ends agreed.
        payload: &'a [u8],
    },
    /// Type `5`. A new request, opening a scope.
    ///
    /// Sent with no scope and no channel — the server mints the scope
    /// in its ack. A client has only this one kind of request, so it
    /// is the only value above `4` a client ever sends.
    Request {
        /// The request bytes.
        payload: &'a [u8],
    },
}

impl<'a> ClientFrame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, scope, channel, payload) = super::split_header(bytes)?;
        Ok(match r#type {
            0 => ClientFrame::Ack { scope, channel },
            1 => ClientFrame::Body {
                scope,
                channel,
                payload,
            },
            2 => ClientFrame::Finish { scope, channel },
            3 => ClientFrame::AuthRequest { payload },
            4 => ClientFrame::AuthResponse { payload },
            5 => ClientFrame::Request { payload },
            other => return Err(FrameError::UnknownType(other)),
        })
    }
}
