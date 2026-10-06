//! One server frame of the daemon connection, as it crosses a wire.

use std::fmt;

use crate::wire::frame::server::ServerFrame;

/// One server frame of the daemon protocol, as the daemon sends it to
/// the program: a response on a scope, the scope's finish, a channel
/// request the daemon opens on a scope, a channel response or its
/// finish answering a channel the program opened.
///
/// Every variant is the wire's own
/// [`ServerFrame`](crate::wire::frame::server::ServerFrame) variant of
/// the same name, with the same members; `Auth` is left out, as it is
/// of the [`Request`](super::super::request::Request). On the wire the
/// server frame's own bytes, so that a bridge forwards bytes and the
/// proxy writes them to the program's socket as they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Served<'a> {
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

impl<'a> From<Served<'a>> for ServerFrame<'a> {
    fn from(served: Served<'a>) -> Self {
        match served {
            Served::Response { scope, payload } => ServerFrame::Response { scope, payload },
            Served::ResponseFinish { scope } => ServerFrame::ResponseFinish { scope },
            Served::ChannelRequest { scope, channel, payload } => ServerFrame::ChannelRequest { scope, channel, payload },
            Served::ChannelResponse { scope, channel, payload } => ServerFrame::ChannelResponse { scope, channel, payload },
            Served::ChannelResponseFinish { scope, channel } => ServerFrame::ChannelResponseFinish { scope, channel },
        }
    }
}

impl<'a> TryFrom<ServerFrame<'a>> for Served<'a> {
    /// The one frame that does not pass.
    type Error = Auth;

    fn try_from(frame: ServerFrame<'a>) -> Result<Self, Auth> {
        Ok(match frame {
            ServerFrame::Auth { .. } => return Err(Auth),
            ServerFrame::Response { scope, payload } => Served::Response { scope, payload },
            ServerFrame::ResponseFinish { scope } => Served::ResponseFinish { scope },
            ServerFrame::ChannelRequest { scope, channel, payload } => Served::ChannelRequest { scope, channel, payload },
            ServerFrame::ChannelResponse { scope, channel, payload } => Served::ChannelResponse { scope, channel, payload },
            ServerFrame::ChannelResponseFinish { scope, channel } => Served::ChannelResponseFinish { scope, channel },
        })
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
