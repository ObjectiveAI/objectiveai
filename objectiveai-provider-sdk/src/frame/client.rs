//! Frames a client sends.
//!
//! A client opens a scope, answers the requests a server makes inside
//! one, opens channels of its own, and — if it was the side that
//! dialled — authenticates.

use super::FrameError;
use crate::agentic_loop;
use crate::images;

/// A frame sent by a client.
///
/// The variants carry only what they can have, so the states that do
/// not exist cannot be built: a reply carries both a scope and the
/// channel it answers on, a scope-opening request carries neither
/// because neither exists until the server answers, and auth comes
/// before there is anything to carry.
///
/// # Two ways to send a request, for now
///
/// [`Request`](Self::Request) is the general one: a scope-opening
/// request whose KIND is a tag inside its payload, the way channel
/// requests already work.
/// [`AgenticLoopRequest`](Self::AgenticLoopRequest) and
/// [`ImagesCheckRequest`](Self::ImagesCheckRequest) are the older
/// shape — one named type apiece, decoded here rather than carried.
///
/// The named pair is on its way out. Once their payloads carry tags
/// they fold into `Request`, this layer stops knowing what an agentic
/// loop is, and [`FrameError::Malformed`] goes with them, since
/// nothing here will parse a payload any more.
///
/// # The gaps
///
/// `2` through `4` are the SCOPE-level replies — the ack that mints a
/// scope, the answer on channel `0`, the finish — and a client sends
/// none of them: it asks for a scope and the server answers in it.
/// Leaving the numbers unused rather than closing up keeps one number
/// meaning one thing in both directions.
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
    /// Type `9`. Run an agent, and stream back what it does.
    ///
    /// The root of everything else: the scope the server mints to
    /// answer it, the channels opened inside that scope, and the
    /// chunks that come back on channel `0`.
    AgenticLoopRequest(agentic_loop::client::request::Frame),
    /// Type `10`. Ask whether the provider can supply an image.
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
            9 => ClientFrame::AgenticLoopRequest(
                serde_json::from_slice(payload)
                    .map_err(FrameError::Malformed)?,
            ),
            10 => ClientFrame::ImagesCheckRequest(
                serde_json::from_slice(payload)
                    .map_err(FrameError::Malformed)?,
            ),
            other => return Err(FrameError::UnknownType(other)),
        })
    }
}
