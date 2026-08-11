//! Why a frame could not be decoded.

/// A frame that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// The bytes ended inside the header.
    Truncated,
    /// A varint ran past 32 bits.
    Overflow,
    /// A `type` no frame in this direction can have.
    ///
    /// Only a CLIENT frame can produce this. A client's types are a
    /// closed set — `0` through `3` — so a fourth value is malformed.
    /// A server's are open above `2`, since each is a kind of request
    /// this layer does not interpret, so an unfamiliar one is a
    /// request from a newer peer rather than an error.
    UnknownType(u8),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated => {
                f.write_str("frame ended inside its header")
            }
            FrameError::Overflow => {
                f.write_str("frame varint exceeded 32 bits")
            }
            FrameError::UnknownType(byte) => {
                write!(f, "unknown client frame type {byte}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// Split a frame's header off the front, returning
/// `(type, scope, channel, payload)`.
///
/// Shared by both directions: the header is the same bytes whoever
/// sent it, and only the MEANING of `type` differs.
pub(super) fn split_header(
    bytes: &[u8],
) -> Result<(u8, u32, u32, &[u8]), FrameError> {
    let (&r#type, rest) = bytes.split_first().ok_or(FrameError::Truncated)?;
    let (scope, n) = super::varint::read(rest)?;
    let rest = &rest[n..];
    let (channel, n) = super::varint::read(rest)?;
    Ok((r#type, scope, channel, &rest[n..]))
}

/// Write a header. Counterpart of [`split_header`].
pub(super) fn write_header(
    r#type: u8,
    scope: u32,
    channel: u32,
    out: &mut Vec<u8>,
) {
    out.push(r#type);
    super::varint::write(scope, out);
    super::varint::write(channel, out);
}

/// How many bytes a header occupies.
pub(super) fn header_len(scope: u32, channel: u32) -> usize {
    1 + super::varint::len(scope) + super::varint::len(channel)
}
