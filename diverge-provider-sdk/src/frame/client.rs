//! Frames a client sends.
//!
//! A client opens a scope, answers the requests a server makes inside
//! one, opens channels of its own, and — if it was the side that
//! dialled — authenticates.

use super::auth::Auth;
use crate::endpoints::ClientRequest;
use super::FrameError;
use crate::decode::Decode;

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
    Auth(
        /// The credential — see [`Auth`].
        Auth<'a>,
    ),
    /// Type `1`. A request that opens a scope.
    ///
    /// The scope is in the header and the CLIENT chose it. Every frame
    /// that follows, in either direction, carries it.
    ///
    /// Read here rather than handed on, because the set is closed
    /// and a server has to know which request it is answering before
    /// it can answer. One this version cannot make out becomes
    /// [`Invalid`](ClientRequest::Invalid), which is answered like any
    /// other rather than refused.
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
        /// Which request — see [`ClientRequest`].
        request: ClientRequest<'a>,
    },
    /// Type `4`. A request to the server, opening a channel.
    ///
    /// The server answers on that same channel with its own responses
    /// and finish.
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

impl<'a> ClientFrame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, scope, channel, payload) = super::split_header(bytes)?;
        Ok(match r#type {
            0 => ClientFrame::Auth(
                Auth::decode(payload).map_err(FrameError::Auth)?,
            ),
            1 => ClientFrame::Request {
                scope,
                // Decoding one is `Infallible`: a payload this version
                // cannot read becomes `Invalid` rather than an error.
                request: ClientRequest::decode(payload)
                    .unwrap_or_else(|error| match error {}),
            },
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
