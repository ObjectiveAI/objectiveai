//! Frames a server sends.
//!
//! A server answers the client's request, answers the channels a
//! client opens, opens channels of its own to ask for things along the
//! way, and — if it was the side that dialled — authenticates. All of
//! it happens inside the scope the client's request opened; a server
//! never initiates one.

use super::FrameError;

/// A frame sent by a server.
///
/// Which of these carry a channel follows from what they answer.
/// [`ResponseAck`](Self::ResponseAck), [`Response`](Self::Response)
/// and [`ResponseFinish`](Self::ResponseFinish) answer the client's
/// own request, which is always channel `0`, so they name none. The
/// channel-level four each name one exchange out of many, so they do.
///
/// # Two channel spaces
///
/// A channel is unique to the side that OPENED it, not to the
/// connection. [`ChannelRequest`](Self::ChannelRequest) mints numbers
/// in the server's space; the channel-level replies quote numbers from
/// the CLIENT's, because they answer channels a client opened. The two
/// spaces never meet, so both ends can count from zero and a channel
/// `5` in one direction has nothing to do with a channel `5` in the
/// other. Which space applies is settled by the frame's direction and
/// its type, and never has to be carried.
///
/// # The gap at `1`
///
/// That is the client's scope-opening request, which a server never
/// sends. It is left empty rather than closed up so a number means one
/// thing whichever way a frame is travelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerFrame<'a> {
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
    /// Type `2` on channel `0`. Acknowledges the client's request and
    /// MINTS its scope — the first frame of the scope, and the only
    /// place the scope comes from.
    ResponseAck {
        /// The newly minted scope.
        scope: u32,
    },
    /// Type `3` on channel `0`. One piece of the answer to the
    /// client's request.
    Response {
        /// The scope.
        scope: u32,
        /// The response bytes.
        payload: &'a [u8],
    },
    /// Type `4` on channel `0`. The scope is over. Nothing bearing it
    /// follows, on any channel.
    ResponseFinish {
        /// The scope.
        scope: u32,
    },
    /// Type `5`. A request to the client, opening a channel.
    ///
    /// The client answers on that same channel with its own channel
    /// ack, responses and finish.
    ///
    /// Not discriminated here. WHICH request this is lives in the
    /// payload's own leading byte — see
    /// [`request::Frame`](crate::agentic_loop::server::request::Frame),
    /// which reads it.
    ChannelRequest {
        /// The scope this happens inside.
        scope: u32,
        /// A channel unique within the scope, minted here in the
        /// SERVER's numbering. Every server request gets its own, so
        /// several can be outstanding at once without their answers
        /// being confusable — and the client's channels are counted
        /// separately and never collide with these.
        channel: u32,
        /// The request bytes, tag included.
        payload: &'a [u8],
    },
    /// Type `6`. Acknowledges a client request; the exchange has
    /// begun.
    ChannelResponseAck {
        /// The scope.
        scope: u32,
        /// The channel of the client request being answered, in the
        /// CLIENT's numbering.
        channel: u32,
    },
    /// Type `7`. One piece of the answer. There may be any number,
    /// including none.
    ChannelResponse {
        /// The scope.
        scope: u32,
        /// The channel of the client request being answered, in the
        /// CLIENT's numbering.
        channel: u32,
        /// The response bytes.
        payload: &'a [u8],
    },
    /// Type `8`. The answer is complete and the channel is closed.
    /// Nothing follows on it.
    ChannelResponseFinish {
        /// The scope.
        scope: u32,
        /// The channel of the client request being answered, in the
        /// CLIENT's numbering.
        channel: u32,
    },
}

impl<'a> ServerFrame<'a> {
    /// Decode one frame. The payload borrows from `bytes`.
    ///
    /// Every type this layer defines is defined here, so an unfamiliar
    /// one is a malformed frame rather than a newer peer — what a
    /// newer peer has more of lives in payloads, behind a
    /// [`ChannelRequest`](Self::ChannelRequest), where this layer
    /// never looks.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, scope, channel, payload) = super::split_header(bytes)?;
        Ok(match r#type {
            // Channel is meaningless on a scope-level reply — it is
            // always `0` — so a non-zero one is ignored rather than
            // rejected.
            0 => ServerFrame::Auth { payload },
            2 => ServerFrame::ResponseAck { scope },
            3 => ServerFrame::Response { scope, payload },
            4 => ServerFrame::ResponseFinish { scope },
            5 => ServerFrame::ChannelRequest {
                scope,
                channel,
                payload,
            },
            6 => ServerFrame::ChannelResponseAck { scope, channel },
            7 => ServerFrame::ChannelResponse {
                scope,
                channel,
                payload,
            },
            8 => ServerFrame::ChannelResponseFinish { scope, channel },
            other => return Err(FrameError::UnknownType(other)),
        })
    }
}
