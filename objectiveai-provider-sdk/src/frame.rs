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
//! because of it. It is minted by the server in the [`Ack`] and lives
//! until the [`Finish`]. `0` means "none yet" — a client request
//! carries it because the scope does not exist until the server
//! answers.
//!
//! **`channel`** is one sub-conversation inside a scope: a single MCP
//! call, or one Postgres connection. `0` is the loop's own chunk
//! stream, which needs no allocation because there is exactly one of
//! it. Only the server opens channels, so there is no odd/even split
//! and no way for the two ends to collide.
//!
//! **`type`** is what the frame IS, not merely what its bytes decode
//! as. That distinction is what lets [`Ack`] and [`Finish`] be types
//! with empty payloads instead of a fourth lifecycle field describing
//! a cross product whose cells are mostly invalid.
//!
//! [`Ack`]: FrameType::Ack
//! [`Finish`]: FrameType::Finish
//!
//! # Why varints
//!
//! `scope` and `channel` are `u64` on the wire only as wide as their
//! value: three bytes of header for a young connection, growing only
//! as the numbers do. Fixed `u32`s would cost nine bytes on every
//! frame, which is noise against a JSON chunk and is not noise against
//! a stream of small Postgres writes.
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
    /// The sub-conversation within the scope. [`LOOP_CHANNEL`] for the
    /// loop's own chunks.
    pub channel: u64,
    /// What this frame is.
    pub r#type: FrameType,
    /// The bytes, if this type carries any.
    pub payload: &'a [u8],
}

/// The scope a client request carries before one exists.
pub const NO_SCOPE: u64 = 0;

/// The channel carrying the loop's own chunk stream.
pub const LOOP_CHANNEL: u64 = 0;

/// What a frame is.
///
/// A `u8` on the wire. Payload-bearing and control frames share one
/// enum because they answer the same question — what is this frame —
/// and splitting them would mean two fields where every combination
/// but a handful is meaningless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    /// An agentic loop request or one of the chunks answering it.
    /// Always on [`LOOP_CHANNEL`].
    AgenticLoop = 0,
    /// A server's MCP request, or the client's reply to it.
    Mcp = 1,
    /// Postgres traffic, in either direction.
    Postgres = 2,
    /// The server acknowledging a client request and minting its
    /// scope. Empty payload — the scope in the header IS the answer.
    Ack = 3,
    /// The scope is over. Nothing follows it bearing that scope.
    /// Empty payload; a failure, if there was one, already arrived as
    /// its own frame.
    Finish = 4,
}

impl FrameType {
    /// The wire byte.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Read a wire byte. `None` for a value this build does not know —
    /// which a receiver should treat as a frame from a newer peer, not
    /// as corruption.
    pub const fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(FrameType::AgenticLoop),
            1 => Some(FrameType::Mcp),
            2 => Some(FrameType::Postgres),
            3 => Some(FrameType::Ack),
            4 => Some(FrameType::Finish),
            _ => None,
        }
    }
}

/// Why a frame could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// The buffer ended inside the header.
    Truncated,
    /// A varint ran past 64 bits.
    Overflow,
    /// A `type` byte this build does not know. Carries the byte, so a
    /// receiver can report what it saw rather than only that it failed.
    UnknownType(u8),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated => f.write_str("frame ended inside its header"),
            FrameError::Overflow => f.write_str("frame varint exceeded 64 bits"),
            FrameError::UnknownType(byte) => {
                write!(f, "unknown frame type {byte}")
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
        let (&byte, payload) =
            rest.split_first().ok_or(FrameError::Truncated)?;
        let r#type =
            FrameType::from_u8(byte).ok_or(FrameError::UnknownType(byte))?;
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
        out.push(self.r#type.as_u8());
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
        // Order matters: the shift itself would panic past 63, so the
        // width check has to short-circuit before the round-trip test.
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
