//! Why a proxy frame could not be decoded.

/// The bytes a frame's header occupies: `type` plus `connection`.
///
/// A constant, for the same reason the main protocol's
/// [`HEADER_LEN`](crate::frame::HEADER_LEN) is one: the payload
/// starts at a known offset rather than wherever a parse happened to
/// finish. Both directions share it — unlike the MCP proxy's wire,
/// the container here has more than one thing to say, so its frames
/// carry a type byte too.
pub const HEADER_LEN: usize = 1 + 4;

/// A frame that could not be read.
///
/// Both are about the envelope — a header too short to read, a type
/// nobody defines. Nothing here is about a payload, because a
/// payload is pgwire and this layer never reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// Fewer than [`HEADER_LEN`] bytes.
    Truncated,
    /// A `type` this wire does not define, in the direction it was
    /// read.
    ///
    /// Every frame kind is fixed and enumerated, so an unfamiliar
    /// value is a malformed frame rather than a peer with more
    /// protocol than this one.
    UnknownType(u8),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated => {
                f.write_str("postgres proxy frame is shorter than its header")
            }
            FrameError::UnknownType(byte) => {
                write!(f, "unknown postgres proxy frame type {byte}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// Split a frame's header off the front, returning
/// `(type, connection, payload)`.
pub(super) fn split_header(
    bytes: &[u8],
) -> Result<(u8, u32, &[u8]), FrameError> {
    let header: &[u8; HEADER_LEN] = bytes
        .get(..HEADER_LEN)
        .and_then(|head| head.try_into().ok())
        .ok_or(FrameError::Truncated)?;
    let connection = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    Ok((header[0], connection, &bytes[HEADER_LEN..]))
}
