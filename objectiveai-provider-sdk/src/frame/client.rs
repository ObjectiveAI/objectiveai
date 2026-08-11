//! Frames a client sends.
//!
//! A client only ever does two things: open a scope, or answer a
//! request the server made inside one. It never opens a channel and
//! never sends a request of its own beyond the first.

use super::FrameError;

/// A frame sent by a client.
///
/// The variants carry only what they can have, so the states that do
/// not exist cannot be built: every reply has both a scope and the
/// channel of the server request it answers, and
/// [`Request`](Self::Request) has neither, because neither exists
/// until the server answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientFrame<'a> {
    /// Type `0`. Acknowledges a server request; the exchange has
    /// begun.
    Ack {
        /// The scope the server minted.
        scope: u64,
        /// The channel of the server request being answered.
        channel: u64,
    },
    /// Type `1`. One piece of the answer. There may be any number,
    /// including none.
    Body {
        /// The scope the server minted.
        scope: u64,
        /// The channel of the server request being answered.
        channel: u64,
        /// The body bytes.
        payload: &'a [u8],
    },
    /// Type `2`. The answer is complete and the channel is closed.
    /// Nothing follows on it.
    Finish {
        /// The scope the server minted.
        scope: u64,
        /// The channel of the server request being answered.
        channel: u64,
    },
    /// Type `3`. A new request, opening a scope.
    ///
    /// Sent with no scope and no channel — the server mints the scope
    /// in its ack. A client has only this one kind of request, so it
    /// is the only value above `2` a client ever sends.
    Request {
        /// The request bytes.
        payload: &'a [u8],
    },
}

impl<'a> ClientFrame<'a> {
    /// Decode one frame. The payload borrows from `bytes`.
    ///
    /// `bytes` is the WebSocket message's binary payload. Nothing here
    /// names a WebSocket type: `axum`'s `Message::Binary` and
    /// tungstenite's both carry `Bytes`, which derefs to `&[u8]`, so
    /// this takes the one thing they agree on and the SDK depends on
    /// neither.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, scope, channel, payload) = super::split_header(bytes)?;
        Ok(match r#type {
            0 => ClientFrame::Ack { scope, channel },
            1 => ClientFrame::Body {
                scope,
                channel,
                payload,
            },
            2 => ClientFrame::Finish { scope, channel },
            3 => ClientFrame::Request { payload },
            other => return Err(FrameError::UnknownType(other)),
        })
    }

    /// `(type, scope, channel, payload)` — the wire header this frame
    /// carries, and its bytes.
    ///
    /// The one place the mapping lives, so the length and the writer
    /// cannot disagree about it.
    fn parts(&self) -> (u8, u64, u64, &'a [u8]) {
        match *self {
            ClientFrame::Ack { scope, channel } => (0, scope, channel, &[]),
            ClientFrame::Body {
                scope,
                channel,
                payload,
            } => (1, scope, channel, payload),
            ClientFrame::Finish { scope, channel } => (2, scope, channel, &[]),
            // A request predates its scope, so both header fields go
            // out as zero and are ignored coming back.
            ClientFrame::Request { payload } => (3, 0, 0, payload),
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
