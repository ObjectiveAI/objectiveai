//! One client frame of the daemon connection, held.

use bytes::Bytes;

use super::Frame;

/// A [`Frame`] that owns its payload: the same four frames, the
/// payload a [`Bytes`] rather than a borrow, for a relay that keeps a
/// frame past the message it arrived in and for the caller's
/// [`Daemon`](crate::provider::client::Daemon), which is handed one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Owned {
    /// See [`Frame::Request`].
    Request {
        /// The scope.
        scope: u32,
        /// The daemon's request frame.
        payload: Bytes,
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
            Owned::Request { scope, payload } => Frame::Request { scope: *scope, payload },
            Owned::ChannelRequest { scope, channel, payload } => Frame::ChannelRequest { scope: *scope, channel: *channel, payload },
            Owned::ChannelResponse { scope, channel, payload } => Frame::ChannelResponse { scope: *scope, channel: *channel, payload },
            Owned::ChannelResponseFinish { scope, channel } => Frame::ChannelResponseFinish { scope: *scope, channel: *channel },
        }
    }

    /// The scope the frame is on.
    pub fn scope(&self) -> u32 {
        match self {
            Owned::Request { scope, .. }
            | Owned::ChannelRequest { scope, .. }
            | Owned::ChannelResponse { scope, .. }
            | Owned::ChannelResponseFinish { scope, .. } => *scope,
        }
    }
}

impl From<Frame<'_>> for Owned {
    /// The payload copied once, out of the message it was borrowed
    /// from.
    fn from(frame: Frame<'_>) -> Self {
        match frame {
            Frame::Request { scope, payload } => Owned::Request { scope, payload: Bytes::copy_from_slice(payload) },
            Frame::ChannelRequest { scope, channel, payload } => Owned::ChannelRequest { scope, channel, payload: Bytes::copy_from_slice(payload) },
            Frame::ChannelResponse { scope, channel, payload } => Owned::ChannelResponse { scope, channel, payload: Bytes::copy_from_slice(payload) },
            Frame::ChannelResponseFinish { scope, channel } => Owned::ChannelResponseFinish { scope, channel },
        }
    }
}
