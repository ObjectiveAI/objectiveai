//! One server frame of the daemon connection, held.

use bytes::Bytes;

use super::Frame;

/// A [`Frame`] that owns its payload: what the caller's
/// [`Daemon`](crate::provider::client::Daemon) hands back, one per
/// frame the daemon's session sends, and what the relay encodes onto
/// the provider's half.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Owned {
    /// See [`Frame::Response`].
    Response {
        /// The scope.
        scope: u32,
        /// The endpoint's response frame.
        payload: Bytes,
    },
    /// See [`Frame::ResponseFinish`].
    ResponseFinish {
        /// The scope.
        scope: u32,
    },
    /// See [`Frame::ChannelRequest`].
    ChannelRequest {
        /// The scope.
        scope: u32,
        /// The channel.
        channel: u32,
        /// The channel request.
        payload: Bytes,
    },
    /// See [`Frame::ChannelResponse`].
    ChannelResponse {
        /// The scope.
        scope: u32,
        /// The channel.
        channel: u32,
        /// The channel response.
        payload: Bytes,
    },
    /// See [`Frame::ChannelResponseFinish`].
    ChannelResponseFinish {
        /// The scope.
        scope: u32,
        /// The channel.
        channel: u32,
    },
}

impl Owned {
    /// The frame, borrowing this one's payload: what goes on a wire.
    pub fn as_frame(&self) -> Frame<'_> {
        match self {
            Owned::Response { scope, payload } => Frame::Response { scope: *scope, payload },
            Owned::ResponseFinish { scope } => Frame::ResponseFinish { scope: *scope },
            Owned::ChannelRequest { scope, channel, payload } => Frame::ChannelRequest { scope: *scope, channel: *channel, payload },
            Owned::ChannelResponse { scope, channel, payload } => Frame::ChannelResponse { scope: *scope, channel: *channel, payload },
            Owned::ChannelResponseFinish { scope, channel } => Frame::ChannelResponseFinish { scope: *scope, channel: *channel },
        }
    }
}

impl From<Frame<'_>> for Owned {
    /// The payload copied once, out of the message it was borrowed
    /// from.
    fn from(frame: Frame<'_>) -> Self {
        match frame {
            Frame::Response { scope, payload } => Owned::Response { scope, payload: Bytes::copy_from_slice(payload) },
            Frame::ResponseFinish { scope } => Owned::ResponseFinish { scope },
            Frame::ChannelRequest { scope, channel, payload } => Owned::ChannelRequest { scope, channel, payload: Bytes::copy_from_slice(payload) },
            Frame::ChannelResponse { scope, channel, payload } => Owned::ChannelResponse { scope, channel, payload: Bytes::copy_from_slice(payload) },
            Frame::ChannelResponseFinish { scope, channel } => Owned::ChannelResponseFinish { scope, channel },
        }
    }
}
