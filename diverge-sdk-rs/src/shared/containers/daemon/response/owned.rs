//! One server frame of the daemon connection, held.

use bytes::Bytes;

use super::Served;

/// A [`Served`] frame that owns its payload: what the caller's
/// [`Daemon`](crate::provider::client::Daemon) hands back, one per
/// frame the daemon's session sends, and what the relay encodes onto
/// the channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Owned {
    /// See [`Served::Response`].
    Response {
        /// The scope.
        scope: u32,
        /// The endpoint's response frame.
        payload: Bytes,
    },
    /// See [`Served::ResponseFinish`].
    ResponseFinish {
        /// The scope.
        scope: u32,
    },
    /// See [`Served::ChannelRequest`].
    ChannelRequest {
        /// The scope.
        scope: u32,
        /// The channel.
        channel: u32,
        /// The channel request.
        payload: Bytes,
    },
    /// See [`Served::ChannelResponse`].
    ChannelResponse {
        /// The scope.
        scope: u32,
        /// The channel.
        channel: u32,
        /// The channel response.
        payload: Bytes,
    },
    /// See [`Served::ChannelResponseFinish`].
    ChannelResponseFinish {
        /// The scope.
        scope: u32,
        /// The channel.
        channel: u32,
    },
}

impl Owned {
    /// The frame, borrowing this one's payload: what goes on a wire.
    pub fn as_served(&self) -> Served<'_> {
        match self {
            Owned::Response { scope, payload } => Served::Response { scope: *scope, payload },
            Owned::ResponseFinish { scope } => Served::ResponseFinish { scope: *scope },
            Owned::ChannelRequest { scope, channel, payload } => Served::ChannelRequest { scope: *scope, channel: *channel, payload },
            Owned::ChannelResponse { scope, channel, payload } => Served::ChannelResponse { scope: *scope, channel: *channel, payload },
            Owned::ChannelResponseFinish { scope, channel } => Served::ChannelResponseFinish { scope: *scope, channel: *channel },
        }
    }
}

impl From<Served<'_>> for Owned {
    /// The payload copied once, out of the message it was borrowed
    /// from.
    fn from(served: Served<'_>) -> Self {
        match served {
            Served::Response { scope, payload } => Owned::Response { scope, payload: Bytes::copy_from_slice(payload) },
            Served::ResponseFinish { scope } => Owned::ResponseFinish { scope },
            Served::ChannelRequest { scope, channel, payload } => Owned::ChannelRequest { scope, channel, payload: Bytes::copy_from_slice(payload) },
            Served::ChannelResponse { scope, channel, payload } => Owned::ChannelResponse { scope, channel, payload: Bytes::copy_from_slice(payload) },
            Served::ChannelResponseFinish { scope, channel } => Owned::ChannelResponseFinish { scope, channel },
        }
    }
}
