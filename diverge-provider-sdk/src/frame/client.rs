//! Frames a client sends.
//!
//! A client opens a scope, answers the requests a server makes inside
//! one, opens channels of its own, and — if it was the side that
//! dialled — authenticates.

use super::FrameError;

/// A frame sent by a client.
///
/// The variants carry only what they can have, so the states that do
/// not exist cannot be built: a reply carries both a scope and the
/// channel it answers on, a scope-opening request carries neither
/// because neither exists until the server answers, and auth comes
/// before there is anything to carry.
///
/// # Nothing here is parsed
///
/// Every payload is bytes, in both directions. This layer splits a
/// header and names a frame kind; what a payload MEANS belongs to the
/// protocol carrying it, and is discriminated by a tag inside the
/// payload rather than by anything out here.
///
/// # The gaps
///
/// `2` through `4` are the SCOPE-level replies — the ack that mints a
/// scope, the answer on channel `0`, the finish — and a client sends
/// none of them: it asks for a scope and the server answers in it.
/// Leaving the numbers unused rather than closing up keeps one number
/// meaning one thing in both directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        /// The credential — an [`Auth`](crate::auth::Auth), which
        /// leads with a mode byte and carries the credential itself
        /// as text.
        ///
        /// Bytes here, like every other payload in this layer. The
        /// frame layer does not read it.
        payload: &'a [u8],
    },
    /// Type `1`. A request that opens a scope.
    ///
    /// Sent with no scope and no channel — the server mints the scope
    /// in its ack, and everything that follows carries it.
    ///
    /// Not discriminated here. WHICH request this is lives in the
    /// payload's own leading byte, so this layer sees one frame kind
    /// and hands the bytes on.
    Request {
        /// The request bytes, tag included.
        payload: &'a [u8],
    },
    /// Type `5`. A request to the server, opening a channel.
    ///
    /// The server answers on that same channel with its own channel
    /// ack, responses and finish.
    ///
    /// Not discriminated here either, for the same reason and by the
    /// same means.
    ChannelRequest {
        /// The scope this happens inside.
        scope: u32,
        /// A channel unique within the scope, minted here in the
        /// CLIENT's numbering. The server's channels are counted
        /// separately and never collide with these.
        channel: u32,
        /// The request bytes, tag included.
        payload: &'a [u8],
    },
    /// Type `6`. Acknowledges a server request; the exchange has
    /// begun.
    ChannelResponseAck {
        /// The scope the server minted.
        scope: u32,
        /// The channel of the server request being answered, in the
        /// SERVER's numbering.
        channel: u32,
    },
    /// Type `7`. One piece of the answer. There may be any number,
    /// including none.
    ChannelResponse {
        /// The scope the server minted.
        scope: u32,
        /// The channel of the server request being answered, in the
        /// SERVER's numbering.
        channel: u32,
        /// The response bytes.
        payload: &'a [u8],
    },
    /// Type `8`. The answer is complete and the channel is closed.
    /// Nothing follows on it.
    ChannelResponseFinish {
        /// The scope the server minted.
        scope: u32,
        /// The channel of the server request being answered, in the
        /// SERVER's numbering.
        channel: u32,
    },
}

impl<'a> ClientFrame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, scope, channel, payload) = super::split_header(bytes)?;
        Ok(match r#type {
            0 => ClientFrame::Auth { payload },
            1 => ClientFrame::Request { payload },
            5 => ClientFrame::ChannelRequest {
                scope,
                channel,
                payload,
            },
            6 => ClientFrame::ChannelResponseAck { scope, channel },
            7 => ClientFrame::ChannelResponse {
                scope,
                channel,
                payload,
            },
            8 => ClientFrame::ChannelResponseFinish { scope, channel },
            other => return Err(FrameError::UnknownType(other)),
        })
    }
}
