//! Frames a server sends.
//!
//! A server answers the client's request, answers the channels a
//! client opens, opens channels of its own to ask for things along the
//! way, and — if it was the side that dialled — authenticates. All of
//! it happens inside the scope the client's request opened; a server
//! never initiates one.

use std::convert::Infallible;

use super::FrameError;
use crate::wire::encode::{Encode, Writer};

/// A frame sent by a server.
///
/// Which of these carry a channel follows from what they answer.
/// [`Response`](Self::Response) and
/// [`ResponseFinish`](Self::ResponseFinish) answer the client's own
/// request, which is always channel `0`, so they name none. The
/// channel-level three each name one exchange out of many, so they
/// do.
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
        /// The credential bytes, mode byte included — see
        /// [`Auth`](super::auth::Auth).
        payload: &'a [u8],
    },
    /// Type `2` on channel `0`. One piece of the answer to the
    /// client's request.
    Response {
        /// The scope.
        scope: u32,
        /// The response bytes.
        payload: &'a [u8],
    },
    /// Type `3` on channel `0`. The scope is over. Nothing bearing it
    /// follows, on any channel.
    ResponseFinish {
        /// The scope.
        scope: u32,
    },
    /// Type `4`. A request to the client, opening a channel.
    ///
    /// The client answers on that same channel with its own responses
    /// and finish.
    ///
    /// One of these per channel, and never a second — see `channel`
    /// below.
    ///
    /// Not discriminated here. WHICH request this is lives in the
    /// payload's own leading byte — see
    /// [`request::Frame`](crate::provider::endpoints::containers::agents::run::server::channel_request::Frame),
    /// which reads it.
    ChannelRequest {
        /// The scope this happens inside.
        scope: u32,
        /// A channel unique within the scope, minted here in the
        /// SERVER's numbering. Every server request gets its own, so
        /// several can be outstanding at once without their answers
        /// being confusable — and the client's channels are counted
        /// separately and never collide with these.
        ///
        /// # One request per channel
        ///
        /// The same rule the client's channels follow, and it is one
        /// rule rather than two conventions: a server sends no further
        /// request on a channel it has opened, and does not use the
        /// number again until the client's response stream on it has
        /// finished.
        ///
        /// It has to hold on both sides to mean anything. A rule that
        /// bound only the client would be a fact about one
        /// implementation, and a reader on either end could no longer
        /// take "this channel" to name one exchange.
        ///
        /// A server with more to say opens another channel — which is
        /// what makes the second half of a duplex exchange possible at
        /// all, since finishing is a responder's act and neither side
        /// can end a channel it is asking on.
        channel: u32,
        /// The request bytes, tag included.
        payload: &'a [u8],
    },
    /// Type `5`. One piece of the answer. There may be any number,
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
    /// Type `6`. The answer is complete and the channel is closed.
    /// Nothing follows on it.
    ChannelResponseFinish {
        /// The scope.
        scope: u32,
        /// The channel of the client request being answered, in the
        /// CLIENT's numbering.
        channel: u32,
    },
}

/// The nine bytes of header, then the payload's own.
///
/// Here rather than in whoever is writing, because the `type` values
/// are here: a writer that spelled them out again would be a second
/// copy of this layer's only vocabulary, kept in step by hand.
///
/// A frame writes its own header, which is the one thing
/// [`Writer`] permits and payloads do not — a
/// writer starts at the buffer's current length, so a frame appending
/// a header and then handing the same writer to its payload is
/// appending twice rather than reaching backwards.
impl Encode for ServerFrame<'_> {
    /// [`Infallible`]: a header is fixed bytes and every payload is
    /// bytes already.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        let (r#type, scope, channel) = match self {
            ServerFrame::Auth { .. } => (0, 0, 0),
            ServerFrame::Response { scope, .. } => (2, *scope, 0),
            ServerFrame::ResponseFinish { scope } => (3, *scope, 0),
            ServerFrame::ChannelRequest { scope, channel, .. } => {
                (4, *scope, *channel)
            }
            ServerFrame::ChannelResponse { scope, channel, .. } => {
                (5, *scope, *channel)
            }
            ServerFrame::ChannelResponseFinish { scope, channel } => {
                (6, *scope, *channel)
            }
        };
        out.extend_from_slice(&[r#type]);
        out.extend_from_slice(&scope.to_be_bytes());
        out.extend_from_slice(&channel.to_be_bytes());
        match self {
            ServerFrame::Auth { payload }
            | ServerFrame::Response { payload, .. }
            | ServerFrame::ChannelRequest { payload, .. }
            | ServerFrame::ChannelResponse { payload, .. } => {
                out.extend_from_slice(payload);
                Ok(())
            }
            ServerFrame::ResponseFinish { .. }
            | ServerFrame::ChannelResponseFinish { .. } => Ok(()),
        }
    }
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
            2 => ServerFrame::Response { scope, payload },
            3 => ServerFrame::ResponseFinish { scope },
            4 => ServerFrame::ChannelRequest {
                scope,
                channel,
                payload,
            },
            5 => ServerFrame::ChannelResponse {
                scope,
                channel,
                payload,
            },
            6 => ServerFrame::ChannelResponseFinish { scope, channel },
            other => return Err(FrameError::UnknownType(other)),
        })
    }
}
