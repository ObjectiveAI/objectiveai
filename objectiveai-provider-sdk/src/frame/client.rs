//! Frames a client sends.
//!
//! A client opens a scope, answers requests the server makes inside
//! one, and — if it was the side that dialled — authenticates. It
//! never opens a channel and never sends a request of its own beyond
//! the first.

use super::FrameError;
use crate::agentic_loop;
use crate::images;

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
/// with a type byte beside it. Two so far —
/// [`AgenticLoopRequest`](Self::AgenticLoopRequest) at `7` and
/// [`ImagesCheckRequest`](Self::ImagesCheckRequest) at `8` — and a
/// third would be a new variant at `9`.
///
/// # The gap at `1` through `3`
///
/// Reserved, and deliberately empty here. Those are the SCOPE-level
/// replies — the ack that mints a scope, the answer on channel `0`,
/// the finish — and a client sends none of them: it asks for a scope
/// and the server answers in it. Leaving the numbers unused rather
/// than closing the gap keeps one number meaning one thing in both
/// directions, which is worth more than three bytes nobody was going
/// to run out of.
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
    /// Type `0`. The first frame of a connection, sent by whichever
    /// side dialled. Nothing may precede it.
    ///
    /// No scope and no channel: nothing has been established yet, and
    /// this is what establishes it. The payload is arbitrary — what
    /// counts as a credential is not this layer's business.
    ///
    /// There is no answer to it. A credential that is accepted is
    /// followed by the connection simply working; one that is not is
    /// followed by a close. A peer that has not authenticated cannot
    /// make the far end compose anything, so a bad credential earns no
    /// bytes to amplify and no reason to read.
    Auth {
        /// The credential, in whatever form the two ends agreed.
        payload: &'a [u8],
    },
    /// Type `4`. Acknowledges a server request; the exchange has
    /// begun.
    ChannelResponseAck {
        /// The scope the server minted.
        scope: u32,
        /// The channel of the server request being answered.
        channel: u32,
    },
    /// Type `5`. One piece of the answer. There may be any number,
    /// including none.
    ChannelResponse {
        /// The scope the server minted.
        scope: u32,
        /// The channel of the server request being answered.
        channel: u32,
        /// The response bytes.
        payload: &'a [u8],
    },
    /// Type `6`. The answer is complete and the channel is closed.
    /// Nothing follows on it.
    ChannelResponseFinish {
        /// The scope the server minted.
        scope: u32,
        /// The channel of the server request being answered.
        channel: u32,
    },
    /// Type `7`. Run an agent, and stream back what it does.
    ///
    /// The request that opens a scope, and the root of everything
    /// else: the scope the server mints to answer it, the channels the
    /// server opens inside that scope, and the chunks that come back
    /// on channel `0`. Sent with neither — the server mints the scope
    /// in its ack.
    AgenticLoopRequest(agentic_loop::client::request::Frame),
    /// Type `8`. Ask whether the provider can supply an image.
    ///
    /// A scope like any other, and a short one: the ack that mints it,
    /// one response on channel `0`, and the finish. The server opens
    /// no channels of its own — there is nothing it needs from the
    /// client to answer.
    ImagesCheckRequest(images::check::client::request::Frame),
}

impl<'a> ClientFrame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, scope, channel, payload) = super::split_header(bytes)?;
        Ok(match r#type {
            0 => ClientFrame::Auth { payload },
            4 => ClientFrame::ChannelResponseAck { scope, channel },
            5 => ClientFrame::ChannelResponse {
                scope,
                channel,
                payload,
            },
            6 => ClientFrame::ChannelResponseFinish { scope, channel },
            7 => ClientFrame::AgenticLoopRequest(
                serde_json::from_slice(payload)
                    .map_err(FrameError::Malformed)?,
            ),
            8 => ClientFrame::ImagesCheckRequest(
                serde_json::from_slice(payload)
                    .map_err(FrameError::Malformed)?,
            ),
            other => return Err(FrameError::UnknownType(other)),
        })
    }
}
