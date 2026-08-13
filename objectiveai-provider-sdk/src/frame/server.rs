//! Frames a server sends.
//!
//! A server answers the client's request, may open channels of its own
//! to ask the client for things along the way, and — if it was the
//! side that dialled — authenticates. The requests and answers all
//! happen inside the scope the client's request opened; a server never
//! initiates one.

use super::FrameError;

/// A frame sent by a server.
///
/// Which of them carry a channel follows from what they answer.
/// [`ResponseAck`](Self::ResponseAck), [`Response`](Self::Response)
/// and [`ResponseFinish`](Self::ResponseFinish) answer the client's
/// own request, which is always channel `0`, so they name none. The
/// channel-level three and [`ChannelRequest`](Self::ChannelRequest)
/// each name one exchange out of many, so they do.
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
    /// Type `1` on channel `0`. Acknowledges the client's request and
    /// MINTS its scope — the first frame of the scope, and the only
    /// place the scope comes from.
    ResponseAck {
        /// The newly minted scope.
        scope: u32,
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
    /// Type `4`. Acknowledges a client request; the exchange has
    /// begun.
    ChannelResponseAck {
        /// The scope.
        scope: u32,
        /// The channel of the client request being answered, in the
        /// CLIENT's numbering.
        channel: u32,
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
    /// Type `7` or above: a request to the client, opening a channel.
    ///
    /// The client answers on that same channel with its own response
    /// ack, responses and response finish.
    ChannelRequest {
        /// The scope this happens inside.
        scope: u32,
        /// A channel unique within the scope, minted here. Every
        /// server request gets its own, so several can be outstanding
        /// at once without their answers being confusable.
        channel: u32,
        /// Which kind of request. `7` or above; what each value means
        /// belongs to the protocol being carried, not to this layer.
        r#type: u8,
        /// The request bytes.
        payload: &'a [u8],
    },
}

impl<'a> ServerFrame<'a> {
    /// Decode one frame. The payload borrows from `bytes`.
    ///
    /// Never returns [`FrameError::UnknownType`]: every value above
    /// `6` is a request whose meaning belongs to the protocol being
    /// carried, so an unfamiliar one is a request from a newer peer
    /// rather than a malformed frame.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, scope, channel, payload) = super::split_header(bytes)?;
        Ok(match r#type {
            // Channel is meaningless on a reply — it is always `0` —
            // so a non-zero one is ignored rather than rejected.
            0 => ServerFrame::Auth { payload },
            1 => ServerFrame::ResponseAck { scope },
            2 => ServerFrame::Response { scope, payload },
            3 => ServerFrame::ResponseFinish { scope },
            4 => ServerFrame::ChannelResponseAck { scope, channel },
            5 => ServerFrame::ChannelResponse {
                scope,
                channel,
                payload,
            },
            6 => ServerFrame::ChannelResponseFinish { scope, channel },
            r#type => ServerFrame::ChannelRequest {
                scope,
                channel,
                r#type,
                payload,
            },
        })
    }
}
