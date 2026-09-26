//! Frames a client sends.
//!
//! A client opens a scope, answers the requests a server makes inside
//! one, opens channels of its own, and — if it was the side that
//! dialled — authenticates.

use std::convert::Infallible;

use super::FrameError;
use crate::wire::encode::{Encode, Writer};

/// A frame sent by a client.
///
/// The variants carry only what they can have, so the states that do
/// not exist cannot be built: a reply carries both a scope and the
/// channel it answers on, a scope-opening request carries the scope it
/// opens and no channel, and auth comes before there is anything to
/// carry.
///
/// # Nothing here is parsed
///
/// Every payload is bytes, without exception. This layer splits a
/// header and names a frame kind; what a payload MEANS belongs to the
/// protocol carrying it, and is discriminated by a tag inside the
/// payload rather than by anything out here.
///
/// Decoding one is a second step the holder takes:
/// [`ClientRequest::decode`](crate::provider::endpoints::ClientRequest) for a
/// request, [`Auth::decode`](super::auth::Auth) for a credential. Which
/// is what lets one type serve both directions of the wire — a sender
/// hands over bytes it encoded itself, and a receiver decodes when it
/// is ready to act, rather than the envelope insisting on both.
///
/// # The gaps
///
/// `2` and `3` are the SCOPE-level replies — the answer on channel
/// `0` and the finish — and a client sends neither: it opens a scope
/// and the server answers in it. Leaving the numbers unused rather
/// than closing up keeps one number meaning one thing in both
/// directions.
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
        /// The credential bytes, mode byte included — see
        /// [`Auth`](super::auth::Auth).
        payload: &'a [u8],
    },
    /// Type `1`. A request that opens a scope.
    ///
    /// The scope is in the header and the CLIENT chose it. Every frame
    /// that follows, in either direction, carries it.
    ///
    /// Handed on rather than read here. A server decodes it when it
    /// goes to answer it, and one this version cannot make out becomes
    /// [`Invalid`](crate::provider::endpoints::ClientRequest::Invalid) at that
    /// point — answered like any other rather than refused.
    Request {
        /// The scope this opens.
        ///
        /// Minted here, because nothing else opens a scope — there is
        /// one minter per connection, so there is nothing to collide
        /// with. It is the same argument that lets both ends count
        /// channels from zero.
        ///
        /// Unique among the scopes this client has open. Reusing one
        /// that has not finished makes two scopes indistinguishable,
        /// and the client is the only party that could have prevented
        /// it. Reuse after a finish is fine, because nothing
        /// remembers.
        scope: u32,
        /// The request bytes, tag included — see
        /// [`ClientRequest`](crate::provider::endpoints::ClientRequest) for what
        /// the tag chooses between.
        payload: &'a [u8],
    },
    /// Type `4`. A request to the server, opening a channel.
    ///
    /// The server answers on that same channel with its own responses
    /// and finish.
    ///
    /// One of these per channel, and never a second — see `channel`
    /// below.
    ///
    /// Not discriminated here either, for the same reason and by the
    /// same means.
    ChannelRequest {
        /// The scope this happens inside.
        scope: u32,
        /// A channel unique within the scope, minted here in the
        /// CLIENT's numbering. The server's channels are counted
        /// separately and never collide with these.
        ///
        /// # One request per channel
        ///
        /// A client sends no further request on a channel it has
        /// opened, and does not use the number again until the
        /// server's response stream on it has finished. Reusing one
        /// that is still live makes two exchanges indistinguishable,
        /// and the client is the only party that could have prevented
        /// it. Reuse after a finish is fine, because nothing
        /// remembers.
        ///
        /// A client with more to say opens another channel. It has
        /// nothing to lose by it — a number is a `u32` and a scope
        /// will not exhaust one — and something to gain: two channels
        /// each end when their answers do, where a channel taking
        /// repeated requests could never say it had stopped asking.
        channel: u32,
        /// The request bytes, tag included.
        payload: &'a [u8],
    },
    /// Type `5`. One piece of the answer. There may be any number,
    /// including none.
    ChannelResponse {
        /// The scope.
        scope: u32,
        /// The channel of the server request being answered, in the
        /// SERVER's numbering.
        channel: u32,
        /// The response bytes.
        payload: &'a [u8],
    },
    /// Type `6`. The answer is complete and the channel is closed.
    /// Nothing follows on it.
    ChannelResponseFinish {
        /// The scope.
        scope: u32,
        /// The channel of the server request being answered, in the
        /// SERVER's numbering.
        channel: u32,
    },
}

/// A frame writes its own header, and a payload never does — see
/// [`ServerFrame`](super::server::ServerFrame) for why the writer
/// permits the one and not the other.
impl Encode for ClientFrame<'_> {
    /// [`Infallible`]: a header is fixed bytes and every payload is
    /// bytes already.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        let (r#type, scope, channel) = match self {
            ClientFrame::Auth { .. } => (0, 0, 0),
            ClientFrame::Request { scope, .. } => (1, *scope, 0),
            ClientFrame::ChannelRequest { scope, channel, .. } => {
                (4, *scope, *channel)
            }
            ClientFrame::ChannelResponse { scope, channel, .. } => {
                (5, *scope, *channel)
            }
            ClientFrame::ChannelResponseFinish { scope, channel } => {
                (6, *scope, *channel)
            }
        };
        out.extend_from_slice(&[r#type]);
        out.extend_from_slice(&scope.to_be_bytes());
        out.extend_from_slice(&channel.to_be_bytes());
        match self {
            ClientFrame::Auth { payload }
            | ClientFrame::Request { payload, .. }
            | ClientFrame::ChannelRequest { payload, .. }
            | ClientFrame::ChannelResponse { payload, .. } => {
                out.extend_from_slice(payload);
                Ok(())
            }
            ClientFrame::ChannelResponseFinish { .. } => Ok(()),
        }
    }
}

impl<'a> ClientFrame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, scope, channel, payload) = super::split_header(bytes)?;
        Ok(match r#type {
            0 => ClientFrame::Auth { payload },
            1 => ClientFrame::Request { scope, payload },
            4 => ClientFrame::ChannelRequest {
                scope,
                channel,
                payload,
            },
            5 => ClientFrame::ChannelResponse {
                scope,
                channel,
                payload,
            },
            6 => ClientFrame::ChannelResponseFinish { scope, channel },
            other => return Err(FrameError::UnknownType(other)),
        })
    }
}
