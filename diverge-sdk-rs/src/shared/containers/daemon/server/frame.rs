//! One server frame of the daemon connection, as it crosses a wire.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;
use crate::wire::frame::server::ServerFrame;

/// One server frame of the daemon protocol, as the daemon sends it to
/// the program: a response on a scope, the scope's finish, a channel
/// request the daemon opens on a scope, a channel response or its
/// finish answering a channel the program opened. One channel response
/// of the provider's half per frame, in the order the daemon sent
/// them, and the proxy writes each to the program's socket as it is.
///
/// Every variant is the wire's own
/// [`ServerFrame`](crate::wire::frame::server::ServerFrame) variant of
/// the same name, with the same members; `Auth` is left out, as it is
/// of the [`client::Frame`](crate::shared::containers::daemon::client::Frame).
///
/// # On the wire
///
/// The server frame's own bytes, with no tag in front, so that a
/// bridge forwards bytes. An `Auth` frame, type `0`, does not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// One response on a scope.
    Response {
        /// The scope.
        scope: u32,
        /// The endpoint's response frame.
        payload: &'a [u8],
    },
    /// The scope's finish: no frame follows it on the scope.
    ResponseFinish {
        /// The scope.
        scope: u32,
    },
    /// A channel the daemon opens on a scope: a content ask.
    ChannelRequest {
        /// The scope.
        scope: u32,
        /// The channel the daemon minted.
        channel: u32,
        /// The channel request.
        payload: &'a [u8],
    },
    /// One response on a channel the program opened.
    ChannelResponse {
        /// The scope.
        scope: u32,
        /// The program's channel.
        channel: u32,
        /// The channel response.
        payload: &'a [u8],
    },
    /// The finish of a channel the program opened.
    ChannelResponseFinish {
        /// The scope.
        scope: u32,
        /// The program's channel.
        channel: u32,
    },
}

impl<'a> From<Frame<'a>> for ServerFrame<'a> {
    fn from(frame: Frame<'a>) -> Self {
        match frame {
            Frame::Response { scope, payload } => ServerFrame::Response { scope, payload },
            Frame::ResponseFinish { scope } => ServerFrame::ResponseFinish { scope },
            Frame::ChannelRequest { scope, channel, payload } => ServerFrame::ChannelRequest { scope, channel, payload },
            Frame::ChannelResponse { scope, channel, payload } => ServerFrame::ChannelResponse { scope, channel, payload },
            Frame::ChannelResponseFinish { scope, channel } => ServerFrame::ChannelResponseFinish { scope, channel },
        }
    }
}

impl<'a> TryFrom<ServerFrame<'a>> for Frame<'a> {
    /// The one frame that does not pass.
    type Error = Auth;

    fn try_from(frame: ServerFrame<'a>) -> Result<Self, Auth> {
        Ok(match frame {
            ServerFrame::Auth { .. } => return Err(Auth),
            ServerFrame::Response { scope, payload } => Frame::Response { scope, payload },
            ServerFrame::ResponseFinish { scope } => Frame::ResponseFinish { scope },
            ServerFrame::ChannelRequest { scope, channel, payload } => Frame::ChannelRequest { scope, channel, payload },
            ServerFrame::ChannelResponse { scope, channel, payload } => Frame::ChannelResponse { scope, channel, payload },
            ServerFrame::ChannelResponseFinish { scope, channel } => Frame::ChannelResponseFinish { scope, channel },
        })
    }
}

/// The server frame's own encoding, and nothing in front of it.
impl Encode for Frame<'_> {
    /// [`Infallible`](std::convert::Infallible): a frame is bytes
    /// copied.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        ServerFrame::from(*self).encode(out)
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// The frame that would not decode, or the one that does not pass.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let frame = ServerFrame::decode(bytes).map_err(FrameError::Frame)?;
        Frame::try_from(frame).map_err(|Auth| FrameError::Auth)
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

/// A server frame of a daemon connection that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Not a server frame: shorter than its header, or of a type the
    /// wire does not have.
    Frame(frame::FrameError),
    /// An `Auth` frame, which does not pass.
    Auth,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Frame(error) => write!(f, "daemon connection server frame did not decode: {error}"),
            FrameError::Auth => f.write_str("an auth frame does not pass on a daemon connection"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Frame(error) => Some(error),
            FrameError::Auth => None,
        }
    }
}
