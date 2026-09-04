//! Why a frame on the `/command` path could not be decoded.

/// The bytes a SERVER frame's header occupies: `type` plus
/// `channel`.
///
/// A constant, for the same reason the main protocol's
/// [`HEADER_LEN`](crate::frame::HEADER_LEN) is one: the payload
/// starts at a known offset rather than wherever a parse happened to
/// finish.
///
/// The container's frame is not counted here: it has one kind, so no
/// type byte, and its header is the channel alone.
pub const HEADER_LEN: usize = 1 + 1;

/// A frame that could not be read.
///
/// Both are about the envelope — a header too short to read, a type
/// nobody defines. A command is bytes and an item is bytes, so
/// neither payload has a failure to report here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// Fewer bytes than a header — a container frame with no channel
    /// byte, or a server frame shorter than [`HEADER_LEN`].
    Truncated,
    /// A `type` this path does not define, on a server frame — the
    /// one direction that has a type byte to be wrong about.
    UnknownType(u8),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated => {
                f.write_str("command frame is shorter than its header")
            }
            FrameError::UnknownType(byte) => {
                write!(f, "unknown command frame type {byte}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// Split a server frame's header off the front, returning
/// `(type, channel, payload)`.
pub(super) fn split_header(
    bytes: &[u8],
) -> Result<(u8, u8, &[u8]), FrameError> {
    let header: &[u8; HEADER_LEN] = bytes
        .get(..HEADER_LEN)
        .and_then(|head| head.try_into().ok())
        .ok_or(FrameError::Truncated)?;
    Ok((header[0], header[1], &bytes[HEADER_LEN..]))
}
