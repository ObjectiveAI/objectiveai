//! Frames a client sends.
//!
//! A client opens a scope, answers requests the server makes inside
//! one, and — if it was the side that dialled — authenticates. It
//! never opens a channel and never sends a request of its own beyond
//! the first.

use super::FrameError;
use crate::agentic_loop::client::request::AgenticLoopRequest;

/// A frame sent by a client.
///
/// The variants carry only what they can have, so the states that do
/// not exist cannot be built: every reply has both a scope and the
/// channel of the server request it answers, and a request has
/// neither, because neither exists until the server answers.
///
/// # Requests are named, not numbered
///
/// A client's requests are a CLOSED set, so they appear here one
/// variant apiece rather than behind a single `Request { payload }`
/// with a type byte beside it. There is exactly one today —
/// [`AgenticLoop`](Self::AgenticLoop), type `5` — and a second would
/// be a new variant at `6`.
///
/// The server's cannot work this way and does not try: its type space
/// is open, so an unfamiliar value there is a request from a newer
/// peer rather than an error, and it keeps its payload as bytes for
/// something above this layer to interpret.
///
/// The consequence is that a client request is DECODED here, not
/// merely carried, which is why decoding one can fail with
/// [`FrameError::Malformed`]. Everything else on this side is still
/// bytes: bodies belong to whatever the channel is tunnelling, and
/// credentials are for the two ends to agree on.
#[derive(Debug, Clone, PartialEq)]
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
    /// Type `5`. Run an agent, and stream back what it does.
    ///
    /// The request that opens a scope, and the root of everything
    /// else: the scope the server mints to answer it, the channels the
    /// server opens inside that scope, and the chunks that come back
    /// on channel `0`. Sent with neither — the server mints the scope
    /// in its ack.
    AgenticLoop(AgenticLoopRequest),
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
            5 => ClientFrame::AgenticLoop(
                serde_json::from_slice(payload)
                    .map_err(FrameError::Malformed)?,
            ),
            other => return Err(FrameError::UnknownType(other)),
        })
    }
}
