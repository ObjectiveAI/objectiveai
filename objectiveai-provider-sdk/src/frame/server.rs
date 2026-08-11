//! Frames a server sends.
//!
//! A server answers the client's request, and may open channels of its
//! own to ask the client for things along the way. Both happen inside
//! the scope the client's request opened — a server never initiates
//! one.

use super::FrameError;

/// A frame sent by a server.
///
/// [`Ack`](Self::Ack), [`Body`](Self::Body) and
/// [`Finish`](Self::Finish) carry no channel: they answer the client's
/// own request, which is always channel `0`. Only
/// [`Request`](Self::Request) names a channel, because it is the only
/// one that opens one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerFrame<'a> {
    /// Type `0` on channel `0`. Acknowledges the client's request and
    /// MINTS its scope — the first frame of the scope, and the only
    /// place the scope comes from.
    Ack {
        /// The newly minted scope.
        scope: u32,
    },
    /// Type `1` on channel `0`. One piece of the answer to the
    /// client's request.
    Body {
        /// The scope.
        scope: u32,
        /// The body bytes.
        payload: &'a [u8],
    },
    /// Type `2` on channel `0`. The scope is over. Nothing bearing it
    /// follows, on any channel.
    Finish {
        /// The scope.
        scope: u32,
    },
    /// Type `3` or above: a request to the client, opening a channel.
    ///
    /// The client answers on that same channel with its own ack, body
    /// and finish.
    Request {
        /// The scope this happens inside.
        scope: u32,
        /// A channel unique within the scope, minted here. Every
        /// server request gets its own, so several can be outstanding
        /// at once without their answers being confusable.
        channel: u32,
        /// Which kind of request. `3` or above; what each value means
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
    /// `2` is a request whose meaning belongs to the protocol being
    /// carried, so an unfamiliar one is a request from a newer peer
    /// rather than a malformed frame.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, scope, channel, payload) = super::split_header(bytes)?;
        Ok(match r#type {
            // Channel is meaningless on a reply — it is always `0` —
            // so a non-zero one is ignored rather than rejected.
            0 => ServerFrame::Ack { scope },
            1 => ServerFrame::Body { scope, payload },
            2 => ServerFrame::Finish { scope },
            r#type => ServerFrame::Request {
                scope,
                channel,
                r#type,
                payload,
            },
        })
    }

    /// `(type, scope, channel, payload)` — the wire header this frame
    /// carries, and its bytes.
    ///
    /// The one place the mapping lives, so the length and the writer
    /// cannot disagree about it.
    fn parts(&self) -> (u8, u32, u32, &'a [u8]) {
        match *self {
            ServerFrame::Ack { scope } => (0, scope, 0, &[]),
            ServerFrame::Body { scope, payload } => (1, scope, 0, payload),
            ServerFrame::Finish { scope } => (2, scope, 0, &[]),
            ServerFrame::Request {
                scope,
                channel,
                r#type,
                payload,
            } => (r#type, scope, channel, payload),
        }
    }

    /// Exactly how many bytes [`Self::encode_into`] will write, so a
    /// caller can size a buffer once and never grow it.
    pub fn encoded_len(&self) -> usize {
        let (_, scope, channel, payload) = self.parts();
        super::header_len(scope, channel) + payload.len()
    }

    /// Append the encoded frame to `out`.
    pub fn encode_into(&self, out: &mut Vec<u8>) {
        let (r#type, scope, channel, payload) = self.parts();
        out.reserve(super::header_len(scope, channel) + payload.len());
        super::write_header(r#type, scope, channel, out);
        out.extend_from_slice(payload);
    }

    /// Encode into a freshly allocated buffer sized exactly. Hand it
    /// to a WebSocket binary message.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.encoded_len());
        self.encode_into(&mut out);
        out
    }
}
