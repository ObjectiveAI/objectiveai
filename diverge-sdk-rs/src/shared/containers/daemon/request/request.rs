//! One client frame of the daemon connection, as it crosses a wire.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;
use crate::wire::frame::client::ClientFrame;

/// One client frame of the daemon protocol, as the program sent it on
/// the proxy's `/daemon`: a request opening a scope, a channel request
/// on one, a channel response or its finish answering a channel the
/// daemon opened on one.
///
/// Every variant is the wire's own
/// [`ClientFrame`](crate::wire::frame::client::ClientFrame) variant of
/// the same name, with the same members. The one the wire has and this
/// does not is `Auth`, which never passes: see
/// [`daemon`](crate::shared::containers::daemon).
///
/// # On the wire
///
/// The client frame's own bytes — its type byte, the scope, the
/// channel, the payload — exactly as the wire lays them out, so that a
/// bridge forwards bytes. An `Auth` frame, type `0`, does not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Request<'a> {
    /// Open a scope with a request: a daemon endpoint's request, tag
    /// and all. Answered with every server frame of the scope.
    Request {
        /// The scope the program minted.
        scope: u32,
        /// The daemon's request frame, as the endpoint lays it out.
        payload: &'a [u8],
    },
    /// Open a channel on a scope: a cancel, a stop. Answered with the
    /// channel's responses and its finish.
    ChannelRequest {
        /// The scope.
        scope: u32,
        /// The channel the program minted.
        channel: u32,
        /// The channel request, as the endpoint lays it out.
        payload: &'a [u8],
    },
    /// Answer a channel the daemon opened on a scope: a piece of an
    /// upload's content. Answered with nothing.
    ChannelResponse {
        /// The scope.
        scope: u32,
        /// The daemon's channel.
        channel: u32,
        /// The channel response, as the endpoint lays it out.
        payload: &'a [u8],
    },
    /// Finish a channel the daemon opened on a scope. Answered with
    /// nothing.
    ChannelResponseFinish {
        /// The scope.
        scope: u32,
        /// The daemon's channel.
        channel: u32,
    },
}

impl<'a> From<Request<'a>> for ClientFrame<'a> {
    fn from(request: Request<'a>) -> Self {
        match request {
            Request::Request { scope, payload } => ClientFrame::Request { scope, payload },
            Request::ChannelRequest { scope, channel, payload } => ClientFrame::ChannelRequest { scope, channel, payload },
            Request::ChannelResponse { scope, channel, payload } => ClientFrame::ChannelResponse { scope, channel, payload },
            Request::ChannelResponseFinish { scope, channel } => ClientFrame::ChannelResponseFinish { scope, channel },
        }
    }
}

impl<'a> TryFrom<ClientFrame<'a>> for Request<'a> {
    /// The one frame that does not pass.
    type Error = Auth;

    fn try_from(frame: ClientFrame<'a>) -> Result<Self, Auth> {
        Ok(match frame {
            ClientFrame::Auth { .. } => return Err(Auth),
            ClientFrame::Request { scope, payload } => Request::Request { scope, payload },
            ClientFrame::ChannelRequest { scope, channel, payload } => Request::ChannelRequest { scope, channel, payload },
            ClientFrame::ChannelResponse { scope, channel, payload } => Request::ChannelResponse { scope, channel, payload },
            ClientFrame::ChannelResponseFinish { scope, channel } => Request::ChannelResponseFinish { scope, channel },
        })
    }
}

/// The client frame's own encoding, and nothing in front of it.
impl Encode for Request<'_> {
    /// [`Infallible`](std::convert::Infallible): a frame is bytes
    /// copied.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        ClientFrame::from(*self).encode(out)
    }
}

impl<'a> Decode<'a> for Request<'a> {
    /// The frame that would not decode, or the one that does not pass.
    type Error = RequestError;

    fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let frame = ClientFrame::decode(bytes).map_err(RequestError::Frame)?;
        Request::try_from(frame).map_err(|Auth| RequestError::Auth)
    }
}

/// An `Auth` frame where a daemon connection's frame was expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Auth;

impl fmt::Display for Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an auth frame does not pass on a daemon connection")
    }
}

impl std::error::Error for Auth {}

/// A daemon connection's ask that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestError {
    /// Not a client frame: shorter than its header, or of a type the
    /// wire does not have.
    Frame(frame::FrameError),
    /// An `Auth` frame, which does not pass.
    Auth,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Frame(error) => write!(f, "daemon connection frame did not decode: {error}"),
            RequestError::Auth => f.write_str("an auth frame does not pass on a daemon connection"),
        }
    }
}

impl std::error::Error for RequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RequestError::Frame(error) => Some(error),
            RequestError::Auth => None,
        }
    }
}
