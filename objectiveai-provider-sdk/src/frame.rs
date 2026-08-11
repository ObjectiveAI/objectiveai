//! The wire frame.
//!
//! Every message is one WebSocket BINARY frame:
//!
//! ```text
//! [scope: varint][channel: varint][type: u8][payload…]
//! ```
//!
//! No length prefix — WebSocket already delimits messages, so carrying
//! one would be paying twice for the same fact.
//!
//! # The three fields
//!
//! **`scope`** is one client request and everything that happens
//! because of it. It is minted by the server, and [`NO_SCOPE`] means
//! "none yet" — a client request carries that because the scope does
//! not exist until the server answers.
//!
//! **`channel`** is one sub-conversation inside a scope. Only the
//! server opens channels, so there is no odd/even split and no way for
//! the two ends to collide.
//!
//! **`type`** is an opaque `u8`. This layer moves frames and does not
//! interpret them: which values exist, which carry payloads, and what
//! those payloads mean all belong to the protocol being carried. A
//! frame with a type this build has never seen is still a
//! well-formed frame, and decoding says so.
//!
//! # Why varints
//!
//! `scope` and `channel` are `u64` on the wire only as wide as their
//! value: three bytes of header for a young connection, growing only
//! as the numbers do. Fixed `u32`s would cost nine bytes on every
//! frame, which is noise against a large payload and is not noise
//! against a stream of small ones.
//!
//! The cost is a real one: there is no constant header length, so the
//! payload offset is parsed rather than known. [`Frame::decode`]
//! returns a borrowed payload, so nothing is copied either way.

/// A decoded frame. The payload borrows from the buffer it was decoded
/// out of — decoding copies nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    /// The client request this belongs to. [`NO_SCOPE`] before the
    /// server has minted one.
    pub scope: u64,
    /// The sub-conversation within the scope.
    pub channel: u64,
    /// What this frame is, as the carried protocol defines it.
    /// Uninterpreted here.
    pub r#type: u8,
    /// The bytes, if this type carries any.
    pub payload: &'a [u8],
}

/// The scope a client request carries before one exists.
pub const NO_SCOPE: u64 = 0;

/// Why a frame could not be decoded.
///
/// Both variants are malformed BYTES. An unrecognized `type` is not
/// here, because it is not a decoding failure — the frame arrived
/// intact and the reader simply does not know what to do with it,
/// which is the carried protocol's problem and not this layer's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// The buffer ended inside the header.
    Truncated,
    /// A varint ran past 64 bits.
    Overflow,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated => {
                f.write_str("frame ended inside its header")
            }
            FrameError::Overflow => {
                f.write_str("frame varint exceeded 64 bits")
            }
        }
    }
}

impl std::error::Error for FrameError {}

impl<'a> Frame<'a> {
    /// Decode one frame. The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (scope, n) = read_varint(bytes)?;
        let rest = &bytes[n..];
        let (channel, n) = read_varint(rest)?;
        let rest = &rest[n..];
        let (&r#type, payload) =
            rest.split_first().ok_or(FrameError::Truncated)?;
        Ok(Frame {
            scope,
            channel,
            r#type,
            payload,
        })
    }

    /// Exactly how many bytes [`Self::encode_into`] will write, so a
    /// caller can size a buffer once and never grow it.
    pub fn encoded_len(&self) -> usize {
        varint_len(self.scope)
            + varint_len(self.channel)
            + 1
            + self.payload.len()
    }

    /// Append the encoded frame to `out`.
    pub fn encode_into(&self, out: &mut Vec<u8>) {
        out.reserve(self.encoded_len());
        write_varint(self.scope, out);
        write_varint(self.channel, out);
        out.push(self.r#type);
        out.extend_from_slice(self.payload);
    }

    /// Encode into a freshly allocated buffer sized exactly.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.encoded_len());
        self.encode_into(&mut out);
        out
    }
}

/// LEB128, unsigned.
fn write_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

/// How many bytes [`write_varint`] will emit for `value`.
fn varint_len(mut value: u64) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }
    len
}

/// Read one LEB128 varint, returning it and how many bytes it took.
fn read_varint(bytes: &[u8]) -> Result<(u64, usize), FrameError> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        let part = u64::from(byte & 0x7F);
        // Order matters: the shift itself would be undefined past 63,
        // so the width check has to short-circuit before the
        // round-trip test.
        if shift >= 64 || (part << shift) >> shift != part {
            return Err(FrameError::Overflow);
        }
        value |= part << shift;
        if byte & 0x80 == 0 {
            return Ok((value, i + 1));
        }
        shift += 7;
    }
    Err(FrameError::Truncated)
}
